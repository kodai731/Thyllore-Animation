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
use crate::vulkanr::renderer::deferred::{compute_projected_bounds_scissor, full_extent_scissor};
use thyllore_effect_core::{
    build_lightning_ubo, compute_lightning_segment_aabb, inverse_view_proj_f64, LightningDebugView,
    LightningSegmentsUBO, LightningUBO, LIGHTNING_MAX_INSTANCES,
};

pub struct LightningPassNode;

/// Every lightning instance the node draws this frame, in UBO slot order.
struct LightningFrame {
    instances: Vec<(LightningUBO, LightningSegmentsUBO)>,
    scissors: Vec<Option<vk::Rect2D>>,
}

impl LightningFrame {
    fn has_visible_instance(&self) -> bool {
        self.scissors.iter().any(Option::is_some)
    }
}

fn lightning_render_settings(app: &App) -> LightningRenderSettings {
    app.data
        .ecs_world
        .get_resource::<LightningRenderSettings>()
        .map(|settings| *settings)
        .unwrap_or_default()
}

pub(super) fn compute_lightning_scissor(
    projection: Option<&ProjectionData>,
    extent: vk::Extent2D,
    debug_view: LightningDebugView,
    effect: &LightningEffect,
    ubo: &LightningUBO,
) -> Option<vk::Rect2D> {
    if debug_view == LightningDebugView::Coverage {
        return Some(full_extent_scissor(extent));
    }
    let corners = compute_lightning_segment_aabb(effect, effect.time)?;
    if debug_view == LightningDebugView::Off && ubo.flash[0] > 0.0 {
        return Some(full_extent_scissor(extent));
    }
    compute_projected_bounds_scissor(projection, extent, &ubo.model, corners)
}

fn lightning_frame(app: &App) -> Option<LightningFrame> {
    let extent = app
        .data
        .ecs_world
        .get_resource::<LightningRenderTargets>()?
        .extent();
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

    let projection = app.data.ecs_world.get_resource::<ProjectionData>();
    let inv_view_proj = projection
        .as_deref()
        .map(|projection| inverse_view_proj_f64(projection.proj, projection.view))
        .unwrap_or_else(cgmath::SquareMatrix::identity);
    let debug_view = lightning_render_settings(app).debug_view;

    let mut instances = Vec::with_capacity(lightnings.len());
    let mut scissors = Vec::with_capacity(lightnings.len());
    for entity in lightnings {
        let Some(effect) = app.data.ecs_world.get_component::<LightningEffect>(entity) else {
            continue;
        };
        let instance = build_lightning_ubo(&effect, inv_view_proj);
        scissors.push(compute_lightning_scissor(
            projection.as_deref(),
            extent,
            debug_view,
            &effect,
            &instance.0,
        ));
        instances.push(instance);
    }
    if instances.is_empty() {
        return None;
    }

    Some(LightningFrame {
        instances,
        scissors,
    })
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
            .filter(LightningFrame::has_visible_instance)
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
    for (slot, ((instance_ubo, instance_segments), scissor)) in
        frame.instances.iter().zip(&frame.scissors).enumerate()
    {
        let Some(scissor) = *scissor else {
            continue;
        };
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
            scissor,
        });
    }
    if draws.is_empty() {
        return Ok(());
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
