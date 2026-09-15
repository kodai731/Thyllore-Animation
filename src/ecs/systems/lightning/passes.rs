use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
use crate::ecs::component::LightningEffect;
use crate::ecs::resource::{
    LightningGpuState, LightningRenderSettings, LightningRenderTargets, ProjectionData,
};
use crate::ecs::systems::lightning::record::{
    record_lightning_resolve_pass, LightningInstanceDraw, LightningPushConstants,
};
use crate::hooks::pass::{
    CoreTarget, PassStage, RenderPassNode, TargetAccess, TargetRef, TargetUse,
};
use thyllore_effect_core::{
    build_lightning_ubo, inverse_view_proj_f64, LightningSegmentsUBO, LightningUBO,
    LIGHTNING_MAX_INSTANCES,
};

pub struct LightningPassNode;

/// Every lightning instance the node draws this frame, in UBO slot order.
struct LightningFrame {
    instances: Vec<(LightningUBO, LightningSegmentsUBO)>,
}

fn lightning_render_settings(app: &App) -> LightningRenderSettings {
    app.data
        .ecs_world
        .get_resource::<LightningRenderSettings>()
        .map(|settings| *settings)
        .unwrap_or_default()
}

fn lightning_frame(app: &App) -> Option<LightningFrame> {
    let gpu_state = app.data.ecs_world.get_resource::<LightningGpuState>()?;
    gpu_state.resolve_pipeline.as_ref()?;
    gpu_state.resolve_descriptor.as_ref()?;
    gpu_state.ubo.as_ref()?;
    gpu_state.segments_ubo.as_ref()?;

    let mut lightnings = app.data.ecs_world.query_lightnings();
    lightnings.truncate(LIGHTNING_MAX_INSTANCES);
    if lightnings.is_empty() {
        return None;
    }

    let inv_view_proj = app
        .data
        .ecs_world
        .get_resource::<ProjectionData>()
        .map(|projection| inverse_view_proj_f64(projection.proj, projection.view))
        .unwrap_or_else(cgmath::SquareMatrix::identity);

    let instances = lightnings
        .into_iter()
        .filter_map(|entity| {
            let effect = app
                .data
                .ecs_world
                .get_component::<LightningEffect>(entity)?;
            Some(build_lightning_ubo(&effect, inv_view_proj))
        })
        .collect::<Vec<_>>();
    if instances.is_empty() {
        return None;
    }

    Some(LightningFrame { instances })
}

impl RenderPassNode for LightningPassNode {
    fn name(&self) -> &'static str {
        "lightning"
    }

    fn stage(&self) -> PassStage {
        PassStage::Effect
    }

    fn writes(&self, app: &App) -> Vec<TargetUse> {
        lightning_frame(app)
            .map(|_| {
                vec![TargetUse::new(
                    TargetRef::Core(CoreTarget::HdrColor),
                    TargetAccess::Attachment {
                        initial_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    },
                )]
            })
            .unwrap_or_default()
    }

    unsafe fn record(
        &self,
        app: &App,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        _frame_slot: usize,
    ) -> Result<()> {
        record_lightning_passes(app, command_buffer, image_index)
    }
}

unsafe fn record_lightning_passes(
    app: &App,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
) -> Result<()> {
    let Some(frame) = lightning_frame(app) else {
        return Ok(());
    };
    let (Some(targets), Some(gpu_state)) = (
        app.data.ecs_world.get_resource::<LightningRenderTargets>(),
        app.data.ecs_world.get_resource::<LightningGpuState>(),
    ) else {
        return Ok(());
    };
    let (Some(pipeline), Some(descriptor), Some(ubo), Some(segments_ubo)) = (
        gpu_state.resolve_pipeline.as_ref(),
        gpu_state.resolve_descriptor.as_ref(),
        gpu_state.ubo.as_ref(),
        gpu_state.segments_ubo.as_ref(),
    ) else {
        return Ok(());
    };
    let ctx = crate::app::build_frame_render_context(app, image_index);

    let mut draws = Vec::with_capacity(frame.instances.len());
    for (slot, (instance_ubo, instance_segments)) in frame.instances.iter().enumerate() {
        ubo.record_update(
            &ctx.device.device,
            command_buffer,
            slot,
            instance_ubo,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        )?;
        segments_ubo.record_update(
            &ctx.device.device,
            command_buffer,
            slot,
            instance_segments,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        )?;
        draws.push(LightningInstanceDraw {
            ubo_dynamic_offset: ubo.slot_offset(slot)? as u32,
            segments_dynamic_offset: segments_ubo.slot_offset(slot)? as u32,
        });
    }

    let settings = lightning_render_settings(app);
    let push_constants = LightningPushConstants::new(
        settings.shading_mode.as_shader_value(),
        settings.reference_step_count as i32,
        settings.debug_view.as_shader_value(),
    );

    record_lightning_resolve_pass(
        &ctx,
        &targets,
        pipeline,
        descriptor,
        &draws,
        push_constants,
        image_index,
        command_buffer,
    )
}
