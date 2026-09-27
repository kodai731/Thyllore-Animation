use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::component::WindTornadoEffect;
use crate::ecs::resource::{ProjectionData, WindGpuState, WindRenderSettings, WindRenderTargets};
use crate::ecs::systems::wind::pipeline::WIND_RESOLVE_OVERLAY;
use crate::ecs::systems::wind::record::record_wind_shadow_bake_pass;
use crate::ecs::PassContext;
use crate::hooks::pass::{
    CoreTarget, PassStage, RenderPassNode, TargetAccess, TargetRef, TargetUse, TransientRequest,
    TransientSlot,
};
use crate::vulkanr::context::RenderTargets;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::renderer::deferred::{compute_bounds_scissor, full_extent_scissor};
use thyllore_effect_core::{
    build_wind_ubo, inverse_view_proj_f64, wind_local_bounds_corners, WindDebugView,
    WindResolveScale, WindShadowSlot, WindUBO, WIND_MAX_INSTANCES,
};
use thyllore_vulkan_core::renderer::{
    union_scissor, OverlayInstanceDraw, OverlayResolveScale, ShadingPushConstants, UPSAMPLE_OVERLAY,
};
use thyllore_vulkan_core::FrameRenderContext;

const REDUCED_COLOR_SLOT: TransientSlot = TransientSlot("wind.reduced_color");

pub struct WindPassNode;

/// Everything the wind node agrees on for one frame. `None` means nothing records this frame.
struct WindFrame {
    ubos: Vec<WindUBO>,
    scissors: Vec<Option<vk::Rect2D>>,
}

impl WindFrame {
    fn has_visible_instance(&self) -> bool {
        self.scissors.iter().any(Option::is_some)
    }
}

fn wind_render_settings(ctx: &PassContext) -> WindRenderSettings {
    ctx.world
        .get_resource::<WindRenderSettings>()
        .map(|settings| *settings)
        .unwrap_or_default()
}

fn wind_frame(ctx: &PassContext) -> Option<WindFrame> {
    let extent = ctx.world.get_resource::<WindRenderTargets>()?.extent();
    let gpu_state = ctx.world.get_resource::<WindGpuState>()?;
    gpu_state.resolve_pipeline.as_ref()?;
    gpu_state.resolve_descriptor.as_ref()?;
    gpu_state.ubo.as_ref()?;

    let mut winds = ctx.world.entities_with::<WindTornadoEffect>();
    winds.truncate(WIND_MAX_INSTANCES);
    if winds.is_empty() {
        return None;
    }

    let inv_view_proj = ctx
        .world
        .get_resource::<ProjectionData>()
        .map(|projection| inverse_view_proj_f64(projection.proj, projection.view));
    let settings = wind_render_settings(ctx);
    let mut ubos = Vec::with_capacity(winds.len());
    let mut scissors = Vec::with_capacity(winds.len());
    for (slot, wind) in winds.into_iter().enumerate() {
        let effect = ctx.world.get_component::<WindTornadoEffect>(wind)?;
        let mut ubo = build_wind_ubo(&effect, WindShadowSlot(slot as u32));
        if let Some(inv_view_proj) = inv_view_proj {
            ubo.inv_view_proj = inv_view_proj;
        }
        let scissor = if settings.debug_view == WindDebugView::Coverage {
            Some(full_extent_scissor(extent))
        } else {
            compute_bounds_scissor(
                ctx.world,
                extent,
                &ubo.model,
                wind_local_bounds_corners(&ubo),
            )
        };
        scissors.push(scissor);
        ubos.push(ubo);
    }

    Some(WindFrame { ubos, scissors })
}

/// The reduced resolution the resolve pass renders at this frame, `None` for the direct HDR path.
fn reduced_scale(ctx: &PassContext) -> Option<OverlayResolveScale> {
    let scale = match wind_render_settings(ctx).resolve_scale {
        WindResolveScale::Full => return None,
        WindResolveScale::Half => OverlayResolveScale::Half,
        WindResolveScale::Quarter => OverlayResolveScale::Quarter,
    };
    wind_frame(ctx)
        .filter(WindFrame::has_visible_instance)
        .map(|_| scale)
}

unsafe fn prepare_reduced_color_target(
    ctx: &mut PassContext,
    scale: OverlayResolveScale,
    frame_slot: usize,
) -> Result<()> {
    let Some((reduced_render_pass, reduced_extent)) = ctx
        .world
        .get_resource::<WindRenderTargets>()
        .map(|targets| {
            (
                targets.reduced.render_pass,
                scale.reduce_extent(targets.extent()),
            )
        })
    else {
        return Ok(());
    };
    let Some(scene_depth_view) = ctx
        .world
        .get_resource::<RenderTargets>()
        .map(|targets| targets.render.gbuffer_depth_image_view)
    else {
        return Ok(());
    };
    let reduced_color = ctx.transient_image(REDUCED_COLOR_SLOT)?;
    ctx.transient.framebuffer(
        &ctx.rrdevice.device,
        reduced_render_pass,
        &[reduced_color.view],
        reduced_extent.width,
        reduced_extent.height,
    )?;

    let Some(mut gpu_state) = ctx.world.get_resource_mut::<WindGpuState>() else {
        return Ok(());
    };
    let generations = vec![reduced_color.generation];
    if gpu_state.upsample_bound.is_bound(frame_slot, &generations) {
        return Ok(());
    }
    if let Some(upsample_descriptor) = gpu_state.upsample_descriptor.as_ref() {
        upsample_descriptor.update_image_views_at(
            ctx.rrdevice,
            frame_slot,
            reduced_color.view,
            scene_depth_view,
        )?;
    }
    gpu_state.upsample_bound.mark_bound(frame_slot, generations);
    Ok(())
}

impl RenderPassNode for WindPassNode {
    fn name(&self) -> &'static str {
        "wind"
    }

    fn stage(&self) -> PassStage {
        PassStage::Effect
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        if wind_frame(ctx)
            .filter(WindFrame::has_visible_instance)
            .is_none()
        {
            return Vec::new();
        }
        let mut uses = vec![TargetUse::new(
            TargetRef::Core(CoreTarget::HdrColor),
            TargetAccess::Attachment {
                initial_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
        )];
        if reduced_scale(ctx).is_some() {
            uses.push(TargetUse::new(
                TargetRef::Transient(REDUCED_COLOR_SLOT),
                TargetAccess::Attachment {
                    initial_layout: vk::ImageLayout::UNDEFINED,
                    final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                },
            ));
        }
        uses
    }

    fn transients(&self, ctx: &PassContext) -> Vec<TransientRequest> {
        let Some(scale) = reduced_scale(ctx) else {
            return Vec::new();
        };
        ctx.world
            .get_resource::<WindRenderTargets>()
            .map(|targets| {
                TransientRequest::new(
                    REDUCED_COLOR_SLOT,
                    targets.reduced.transient_desc(scale, targets.extent()),
                )
            })
            .into_iter()
            .collect()
    }

    unsafe fn prepare(&self, ctx: &mut PassContext, frame_slot: usize) -> Result<()> {
        let Some(scale) = reduced_scale(ctx) else {
            return Ok(());
        };
        prepare_reduced_color_target(ctx, scale, frame_slot)
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        frame_slot: usize,
    ) -> Result<()> {
        record_wind_passes(ctx, command_buffer, image_index, frame_slot)
    }
}

unsafe fn record_wind_passes(
    ctx: &PassContext,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
    frame_slot: usize,
) -> Result<()> {
    let Some(frame) = wind_frame(ctx) else {
        return Ok(());
    };
    let (Some(wind_targets), Some(gpu_state)) = (
        ctx.world.get_resource::<WindRenderTargets>(),
        ctx.world.get_resource::<WindGpuState>(),
    ) else {
        return Ok(());
    };
    let (Some(shading_pipeline), Some(descriptor), Some(wind_ubo)) = (
        gpu_state.resolve_pipeline.as_ref(),
        gpu_state.resolve_descriptor.as_ref(),
        gpu_state.ubo.as_ref(),
    ) else {
        return Ok(());
    };
    let wind_buffer = &*wind_targets;
    let render = ctx.frame_render_context(image_index);

    let settings = wind_render_settings(ctx);
    let push_constants = ShadingPushConstants::new(
        settings.shading_mode.as_shader_value(),
        settings.reference_step_count as i32,
        settings.debug_view.as_shader_value(),
    );

    let mut draws = Vec::with_capacity(frame.ubos.len());
    for (slot, (ubo, scissor)) in frame.ubos.iter().zip(&frame.scissors).enumerate() {
        let Some(scissor) = *scissor else {
            continue;
        };
        wind_ubo.record_update(
            &render.device.device,
            command_buffer,
            slot,
            ubo,
            vk::PipelineStageFlags::COMPUTE_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
        )?;
        draws.push(OverlayInstanceDraw {
            dynamic_offsets: vec![wind_ubo.slot_offset(slot)? as u32],
            scissor,
        });
    }
    if draws.is_empty() {
        return Ok(());
    }

    if let (Some(bake_pipeline), Some(bake_descriptor)) = (
        gpu_state.shadow_bake_pipeline.as_ref(),
        gpu_state.shadow_bake_descriptor.as_ref(),
    ) {
        record_wind_shadow_bake_pass(
            &render,
            wind_buffer,
            bake_pipeline,
            bake_descriptor,
            &draws,
            image_index,
            command_buffer,
        )?;
    }

    let resolve_sets = [
        render.graphics.frame_set.sets[image_index],
        descriptor.descriptor_set(),
    ];
    match reduced_scale(ctx) {
        None => {
            let pass = wind_buffer.overlay_pass();
            for draw in &draws {
                WIND_RESOLVE_OVERLAY.record(
                    &render.device.device,
                    command_buffer,
                    &pass,
                    draw.scissor,
                    shading_pipeline,
                    Some(push_constants.as_bytes()),
                    &resolve_sets,
                    std::slice::from_ref(draw),
                )?;
            }
        }
        Some(scale) => record_reduced_scale_wind_passes(
            ctx,
            &gpu_state,
            &render,
            wind_buffer,
            scale,
            shading_pipeline,
            &resolve_sets,
            &draws,
            push_constants,
            frame_slot,
            command_buffer,
        )?,
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
unsafe fn record_reduced_scale_wind_passes(
    pass_ctx: &PassContext,
    gpu_state: &WindGpuState,
    ctx: &FrameRenderContext,
    wind_buffer: &WindRenderTargets,
    scale: OverlayResolveScale,
    shading_pipeline: &RRPipeline,
    resolve_sets: &[vk::DescriptorSet],
    draws: &[OverlayInstanceDraw],
    push_constants: ShadingPushConstants,
    frame_slot: usize,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    let (Some(upsample_pipeline), Some(upsample_descriptor)) = (
        gpu_state.upsample_pipeline.as_ref(),
        gpu_state.upsample_descriptor.as_ref(),
    ) else {
        return Ok(());
    };
    let reduced_color = pass_ctx.transient_image(REDUCED_COLOR_SLOT)?;
    let Some(reduced_framebuffer) = pass_ctx
        .transient
        .cached_framebuffer(wind_buffer.reduced.render_pass, &[reduced_color.view])
    else {
        return Ok(());
    };
    let device = &ctx.device.device;

    let reduced_pass =
        wind_buffer
            .reduced
            .overlay_pass(reduced_framebuffer, scale, wind_buffer.extent());
    let reduced_draws: Vec<OverlayInstanceDraw> = draws
        .iter()
        .map(|draw| OverlayInstanceDraw {
            dynamic_offsets: draw.dynamic_offsets.clone(),
            scissor: scale.reduce_scissor(draw.scissor, reduced_pass.extent),
        })
        .collect();
    WIND_RESOLVE_OVERLAY.record(
        device,
        command_buffer,
        &reduced_pass,
        reduced_pass.full_area(),
        shading_pipeline,
        Some(push_constants.as_bytes()),
        resolve_sets,
        &reduced_draws,
    )?;

    let upsample_scissor =
        union_scissor(draws.iter().map(|draw| draw.scissor), wind_buffer.extent());
    UPSAMPLE_OVERLAY.record(
        device,
        command_buffer,
        &wind_buffer.overlay_pass(),
        upsample_scissor,
        upsample_pipeline,
        None,
        &[upsample_descriptor.descriptor_set(frame_slot)],
        &[OverlayInstanceDraw {
            dynamic_offsets: Vec::new(),
            scissor: upsample_scissor,
        }],
    )
}
