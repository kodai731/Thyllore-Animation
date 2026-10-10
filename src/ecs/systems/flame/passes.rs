use anyhow::Result;
use cgmath::{InnerSpace, SquareMatrix, Vector3};
use vulkanalia::prelude::v1_0::*;

use crate::ecs::component::FlameEffect;
use crate::ecs::systems::flame::pipeline::FLAME_OVERLAY;
use crate::ecs::world::Entity;
use crate::ecs::PassContext;
use crate::hooks::pass::{
    CoreTarget, PassStage, RenderPassNode, ShaderStage, TargetAccess, TargetRef, TargetUse,
};
use crate::vulkanr::renderer::deferred::compute_bounds_scissor;
use thyllore_vulkan_core::renderer::{
    OverlayAttachmentLoad, OverlayInstanceDraw, ShadingPushConstants,
};
use thyllore_vulkan_core::resource::RenderTargetKey;

const HISTORY_KEYS: [RenderTargetKey; 2] = [
    RenderTargetKey::EffectHistory(0),
    RenderTargetKey::EffectHistory(1),
];

pub struct FlamePassNode;

fn flame_history_index(ctx: &PassContext, flames: &[Entity]) -> usize {
    flames
        .first()
        .and_then(|first| {
            ctx.world
                .get_component::<crate::ecs::component::FlameTemporalAccum>(*first)
        })
        .map(|temporal| (temporal.frame_index as usize) & 1)
        .unwrap_or(0)
}

/// Everything the flame node agrees on for one frame. `None` means nothing records this frame.
struct FlameFrame {
    ubos: Vec<thyllore_effect_core::FlameUBO>,
    scissors: Vec<Option<vk::Rect2D>>,
    history_index: usize,
}

impl FlameFrame {
    fn has_visible_instance(&self) -> bool {
        self.scissors.iter().any(Option::is_some)
    }
}

fn flame_frame(ctx: &PassContext) -> Option<FlameFrame> {
    let extent = ctx
        .world
        .get_resource::<crate::ecs::resource::FlameRenderTargets>()?
        .history
        .extent();
    let gpu_state = ctx
        .world
        .get_resource::<crate::ecs::resource::FlameGpuState>()?;
    gpu_state.shading_pipeline.as_ref()?;
    gpu_state.descriptor.as_ref()?;
    gpu_state.ubo.as_ref()?;

    let mut flames = ctx.world.entities_with::<FlameEffect>();
    sort_flames_back_to_front(ctx, &mut flames);
    flames.truncate(thyllore_effect_core::FLAME_MAX_INSTANCES);
    if flames.is_empty() {
        return None;
    }

    let ubos: Vec<_> = flames
        .iter()
        .map(|flame| build_instance_ubo(ctx, *flame))
        .collect::<Option<_>>()?;
    let scissors = ubos
        .iter()
        .map(|ubo| instance_scissor(ctx, extent, ubo))
        .collect();
    let history_index = flame_history_index(ctx, &flames);

    Some(FlameFrame {
        ubos,
        scissors,
        history_index,
    })
}

fn sort_flames_back_to_front(ctx: &PassContext, flames: &mut [Entity]) {
    let Some(projection) = ctx
        .world
        .get_resource::<crate::ecs::resource::ProjectionData>()
    else {
        return;
    };
    let view_inverse = projection
        .view
        .invert()
        .unwrap_or_else(cgmath::Matrix4::identity);
    let camera_pos = Vector3::new(view_inverse[3][0], view_inverse[3][1], view_inverse[3][2]);

    let camera_distance = |flame: &Entity| {
        ctx.world
            .get_component::<crate::ecs::component::FlameEffect>(*flame)
            .map(|effect| {
                let position =
                    Vector3::new(effect.position[0], effect.position[1], effect.position[2]);
                (position - camera_pos).magnitude()
            })
    };
    flames.sort_by(|a, b| match (camera_distance(a), camera_distance(b)) {
        (Some(dist_a), Some(dist_b)) => dist_b
            .partial_cmp(&dist_a)
            .unwrap_or(std::cmp::Ordering::Equal),
        _ => std::cmp::Ordering::Equal,
    });
}

fn build_instance_ubo(ctx: &PassContext, flame: Entity) -> Option<thyllore_effect_core::FlameUBO> {
    let world = &ctx.world;
    let effect = world.get_component::<crate::ecs::component::FlameEffect>(flame)?;
    let trail = world.get_component::<crate::ecs::component::FlameTrail>(flame);
    let is_noise_mode = world
        .get_resource::<crate::ecs::resource::FlameRenderSettings>()
        .map(|s| s.shading_mode == thyllore_effect_core::FlameShadingMode::NoiseRaymarch)
        .unwrap_or(false);
    let baked = world
        .get_component::<crate::ecs::component::FlameBaked>(flame)
        .cloned()
        .unwrap_or_default();
    let temporal_accum = world
        .get_component::<crate::ecs::component::FlameTemporalAccum>(flame)
        .cloned()
        .unwrap_or_default();

    Some(thyllore_effect_core::build_flame_ubo_with_trail(
        &effect,
        &baked,
        &temporal_accum,
        trail.as_deref().map(|t| &t.state),
        is_noise_mode,
    ))
}

fn instance_scissor(
    ctx: &PassContext,
    extent: vk::Extent2D,
    ubo: &thyllore_effect_core::FlameUBO,
) -> Option<vk::Rect2D> {
    let bend_offset = [
        ubo.wind_bend.wind_direction[0] * ubo.wind_bend.bend_amount,
        ubo.wind_bend.wind_direction[1] * ubo.wind_bend.bend_amount,
    ];
    let support_scale = thyllore_effect_core::flame_shell_support_scale(
        ubo.emitter_params.kind as u32,
        ubo.emitter_params.ring_major_ratio,
        ubo.support_motion.support_margin,
    );
    let bounds = thyllore_effect_core::flame_local_bounds(
        bend_offset,
        support_scale,
        ubo.support_motion.support_margin,
        thyllore_effect_core::FlameProxyPad {
            radial: thyllore_effect_core::flame_proxy_radial_pad(
                ubo.branch_field.bounding_pad,
                ubo.support_motion.meander_amp,
            ),
            top: ubo.branch_field.bounding_pad_y,
        },
    );
    compute_bounds_scissor(
        ctx.world,
        extent,
        &ubo.model,
        thyllore_effect_core::flame_local_bounds_corners(&bounds),
    )
}

impl RenderPassNode for FlamePassNode {
    fn name(&self) -> &'static str {
        "flame"
    }

    fn stage(&self) -> PassStage {
        PassStage::Effect
    }

    fn reads(&self, ctx: &PassContext) -> Vec<TargetUse> {
        flame_frame(ctx)
            .filter(FlameFrame::has_visible_instance)
            .map(|frame| frame.history_index)
            .map(|history_index| {
                TargetUse::new(
                    TargetRef::Storage(HISTORY_KEYS[1 - history_index]),
                    TargetAccess::Sampled(ShaderStage::Fragment),
                )
            })
            .into_iter()
            .collect()
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        flame_frame(ctx)
            .filter(FlameFrame::has_visible_instance)
            .map(|frame| frame.history_index)
            .map(|history_index| {
                vec![
                    TargetUse::new(
                        TargetRef::Core(CoreTarget::HdrColor),
                        TargetAccess::Attachment {
                            initial_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                            final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        },
                    ),
                    TargetUse::new(
                        TargetRef::Storage(HISTORY_KEYS[history_index]),
                        TargetAccess::Attachment {
                            initial_layout: vk::ImageLayout::UNDEFINED,
                            final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        },
                    ),
                ]
            })
            .unwrap_or_default()
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        _frame_slot: usize,
    ) -> Result<()> {
        record_flame_passes(ctx, command_buffer, image_index)
    }
}

unsafe fn record_flame_passes(
    ctx: &PassContext,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
) -> Result<()> {
    let Some(frame) = flame_frame(ctx) else {
        return Ok(());
    };
    let (Some(flame_targets), Some(gpu_state)) = (
        ctx.world
            .get_resource::<crate::ecs::resource::FlameRenderTargets>(),
        ctx.world
            .get_resource::<crate::ecs::resource::FlameGpuState>(),
    ) else {
        return Ok(());
    };
    let (Some(shading_pipeline), Some(descriptor), Some(flame_ubo)) = (
        gpu_state.shading_pipeline.as_ref(),
        gpu_state.descriptor.as_ref(),
        gpu_state.ubo.as_ref(),
    ) else {
        return Ok(());
    };
    let render = ctx.frame_render_context(image_index);
    let device = &render.device.device;

    let settings = ctx
        .world
        .get_resource::<crate::ecs::resource::FlameRenderSettings>()
        .map(|settings| *settings)
        .unwrap_or_default();
    let push_constants = ShadingPushConstants::new(
        settings.shading_mode.as_shader_value(),
        settings.resolved_step_count() as i32,
        settings.debug_view.as_shader_value(),
    );

    let mut draws = Vec::with_capacity(frame.ubos.len());
    for (slot, (ubo, scissor)) in frame.ubos.iter().zip(&frame.scissors).enumerate() {
        let Some(scissor) = *scissor else {
            continue;
        };
        flame_ubo.record_update(
            device,
            command_buffer,
            slot,
            ubo,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        )?;
        draws.push(OverlayInstanceDraw {
            dynamic_offsets: vec![flame_ubo.slot_offset(slot)? as u32],
            scissor,
        });
    }
    if draws.is_empty() {
        return Ok(());
    }

    let pass = flame_targets
        .history
        .overlay_pass(frame.history_index, OverlayAttachmentLoad::Clear([0.0; 4]));
    FLAME_OVERLAY.record(
        device,
        command_buffer,
        &pass,
        pass.full_area(),
        shading_pipeline,
        Some(push_constants.as_bytes()),
        &[
            render.graphics.frame_set.sets[image_index],
            descriptor.descriptor_sets[frame.history_index],
        ],
        &draws,
    )
}
