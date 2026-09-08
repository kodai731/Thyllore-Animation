use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
use crate::ecs::component::WindTornadoEffect;
use crate::ecs::resource::{ProjectionData, WindRenderSettings, WindRenderTargets};
use crate::hooks::pass::{
    CoreTarget, PassStage, RenderPassNode, TargetAccess, TargetRef, TargetUse,
};
use crate::vulkanr::renderer::deferred::{compute_bounds_scissor, full_extent_scissor};
use thyllore_effect_core::{
    build_wind_ubo, inverse_view_proj_f64, wind_local_bounds_corners, WindDebugView,
    WindResolveScale, WindShellParams, WindUBO,
};
use thyllore_vulkan_core::renderer::WindInstanceDraw;

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

fn wind_render_settings(app: &App) -> WindRenderSettings {
    app.data
        .ecs_world
        .get_resource::<WindRenderSettings>()
        .map(|settings| *settings)
        .unwrap_or_default()
}

fn wind_frame(app: &App) -> Option<WindFrame> {
    let extent = app
        .data
        .ecs_world
        .get_resource::<WindRenderTargets>()?
        .buffer
        .extent();
    app.data.raytracing.wind_shading_pipeline.as_ref()?;
    app.data.raytracing.wind_descriptor.as_ref()?;
    app.data.raytracing.wind_ubo.as_ref()?;

    let mut winds = app.data.ecs_world.query_winds();
    winds.truncate(thyllore_vulkan_core::resource::MAX_WIND_INSTANCES);
    if winds.is_empty() {
        return None;
    }

    let inv_view_proj = app
        .data
        .ecs_world
        .get_resource::<ProjectionData>()
        .map(|projection| inverse_view_proj_f64(projection.proj, projection.view));
    let settings = wind_render_settings(app);
    let mut ubos = Vec::with_capacity(winds.len());
    let mut scissors = Vec::with_capacity(winds.len());
    for wind in winds {
        let effect = app
            .data
            .ecs_world
            .get_component::<WindTornadoEffect>(wind)?;
        let mut ubo = build_wind_ubo(&effect);
        if let Some(inv_view_proj) = inv_view_proj {
            ubo.inv_view_proj = inv_view_proj;
        }
        let params = WindShellParams::from_effect(&effect);
        let scissor = if settings.debug_view == WindDebugView::Coverage {
            Some(full_extent_scissor(extent))
        } else {
            compute_bounds_scissor(app, extent, &ubo.model, wind_local_bounds_corners(&params))
        };
        scissors.push(scissor);
        ubos.push(ubo);
    }

    Some(WindFrame { ubos, scissors })
}

impl RenderPassNode for WindPassNode {
    fn name(&self) -> &'static str {
        "wind"
    }

    fn stage(&self) -> PassStage {
        PassStage::Effect
    }

    fn writes(&self, app: &App) -> Vec<TargetUse> {
        wind_frame(app)
            .filter(WindFrame::has_visible_instance)
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
        record_wind_passes(app, command_buffer, image_index)
    }
}

unsafe fn record_wind_passes(
    app: &App,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
) -> Result<()> {
    let Some(frame) = wind_frame(app) else {
        return Ok(());
    };
    let (Some(wind_targets), Some(shading_pipeline), Some(descriptor), Some(wind_ubo)) = (
        app.data.ecs_world.get_resource::<WindRenderTargets>(),
        app.data.raytracing.wind_shading_pipeline.as_ref(),
        app.data.raytracing.wind_descriptor.as_ref(),
        app.data.raytracing.wind_ubo.as_ref(),
    ) else {
        return Ok(());
    };
    let wind_buffer = &wind_targets.buffer;
    let ctx = crate::ecs::systems::phases::build_frame_render_context(app, image_index);

    let settings = wind_render_settings(app);
    let push_constants = thyllore_vulkan_core::renderer::WindPushConstants::new(
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
            &ctx.device.device,
            command_buffer,
            slot,
            ubo,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        )?;
        draws.push(WindInstanceDraw {
            ubo_dynamic_offset: wind_ubo.slot_offset(slot)? as u32,
            scissor,
        });
    }
    if draws.is_empty() {
        return Ok(());
    }

    match settings.resolve_scale {
        WindResolveScale::Full => {
            for draw in &draws {
                thyllore_vulkan_core::renderer::record_wind_shading_pass(
                    &ctx,
                    wind_buffer,
                    shading_pipeline,
                    descriptor,
                    draw.ubo_dynamic_offset,
                    draw.scissor,
                    push_constants,
                    image_index,
                    command_buffer,
                )?;
            }
        }
        WindResolveScale::Half => record_half_scale_wind_passes(
            app,
            &ctx,
            wind_buffer,
            shading_pipeline,
            descriptor,
            &draws,
            push_constants,
            image_index,
            command_buffer,
        )?,
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
unsafe fn record_half_scale_wind_passes(
    app: &App,
    ctx: &thyllore_vulkan_core::FrameRenderContext,
    wind_buffer: &thyllore_vulkan_core::resource::WindBuffer,
    shading_pipeline: &thyllore_vulkan_core::pipeline::RRPipeline,
    descriptor: &thyllore_vulkan_core::descriptor::RRWindDescriptorSet,
    draws: &[WindInstanceDraw],
    push_constants: thyllore_vulkan_core::renderer::WindPushConstants,
    image_index: usize,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    let (Some(upsample_pipeline), Some(upsample_descriptor)) = (
        app.data.raytracing.wind_upsample_pipeline.as_ref(),
        app.data.raytracing.wind_upsample_descriptor.as_ref(),
    ) else {
        return Ok(());
    };

    let half_extent = wind_buffer.half_extent();
    let half_draws: Vec<WindInstanceDraw> = draws
        .iter()
        .map(|draw| WindInstanceDraw {
            ubo_dynamic_offset: draw.ubo_dynamic_offset,
            scissor: halved_scissor(draw.scissor, half_extent),
        })
        .collect();

    thyllore_vulkan_core::renderer::record_wind_half_resolve_pass(
        ctx,
        wind_buffer,
        shading_pipeline,
        descriptor,
        &half_draws,
        push_constants,
        image_index,
        command_buffer,
    )?;
    thyllore_vulkan_core::renderer::record_wind_upsample_pass(
        ctx,
        wind_buffer,
        upsample_pipeline,
        upsample_descriptor,
        union_scissor(draws.iter().map(|draw| draw.scissor), wind_buffer.extent()),
        command_buffer,
    )
}

/// Half resolution scissor grown by one texel so the upsample taps stay inside resolved pixels.
fn halved_scissor(scissor: vk::Rect2D, half_extent: vk::Extent2D) -> vk::Rect2D {
    let left = (scissor.offset.x / 2 - 1).max(0);
    let top = (scissor.offset.y / 2 - 1).max(0);
    let right = ((scissor.offset.x + scissor.extent.width as i32).div_euclid(2) + 2)
        .min(half_extent.width as i32);
    let bottom = ((scissor.offset.y + scissor.extent.height as i32).div_euclid(2) + 2)
        .min(half_extent.height as i32);

    vk::Rect2D {
        offset: vk::Offset2D { x: left, y: top },
        extent: vk::Extent2D {
            width: (right - left).max(0) as u32,
            height: (bottom - top).max(0) as u32,
        },
    }
}

fn union_scissor(
    scissors: impl Iterator<Item = vk::Rect2D>,
    full_extent: vk::Extent2D,
) -> vk::Rect2D {
    let mut bounds: Option<(i32, i32, i32, i32)> = None;
    for scissor in scissors {
        let candidate = (
            scissor.offset.x,
            scissor.offset.y,
            scissor.offset.x + scissor.extent.width as i32,
            scissor.offset.y + scissor.extent.height as i32,
        );
        bounds = Some(match bounds {
            Some(current) => (
                current.0.min(candidate.0),
                current.1.min(candidate.1),
                current.2.max(candidate.2),
                current.3.max(candidate.3),
            ),
            None => candidate,
        });
    }

    let Some((left, top, right, bottom)) = bounds else {
        return full_extent_scissor(full_extent);
    };
    let left = (left - 2).max(0);
    let top = (top - 2).max(0);
    let right = (right + 2).min(full_extent.width as i32);
    let bottom = (bottom + 2).min(full_extent.height as i32);

    vk::Rect2D {
        offset: vk::Offset2D { x: left, y: top },
        extent: vk::Extent2D {
            width: (right - left).max(0) as u32,
            height: (bottom - top).max(0) as u32,
        },
    }
}
