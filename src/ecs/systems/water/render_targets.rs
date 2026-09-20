use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::{WaterGpuState, WaterRenderTargets};
use crate::ecs::systems::raytracing_systems::ensure_effect_trace_pipeline;
use crate::ecs::{EffectContext, MAX_FRAMES_IN_FLIGHT};
use crate::hooks::effect::EffectHook;
use crate::vulkanr::context::RenderTargets;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::GpuResource;
use thyllore_vulkan_core::resource::hdr_buffer::HDR_FORMAT;
use thyllore_vulkan_core::resource::render_target_storage::RenderTargetKey;
use thyllore_vulkan_core::resource::{AccumulationTarget, HistoryTargets, HistoryTargetsDesc};

const WATER_HISTORY_KEYS: [RenderTargetKey; 2] = [
    RenderTargetKey::EffectHistory(2),
    RenderTargetKey::EffectHistory(3),
];
const WATER_CAUSTIC_ACCUM_KEY: RenderTargetKey = RenderTargetKey::EffectAccumulation(0);
const WATER_CAUSTIC_ACCUM_FORMAT: vk::Format = vk::Format::R32_UINT;

pub const WATER_EFFECT_HOOK: EffectHook = EffectHook {
    name: "water",
    setup: Some(setup_water),
    after_overrides: None,
    on_viewport_resize: Some(resize_water_render_targets),
    passes: super::passes::WATER_PASS_NODES,
};

unsafe fn create_water_render_targets(
    ctx: &mut EffectContext,
    depth_view: vk::ImageView,
) -> Result<bool> {
    let Some(hdr_view) = ctx.hdr_color_view else {
        return Ok(false);
    };

    let history = HistoryTargets::new(
        ctx.instance,
        ctx.rrdevice,
        ctx.storage,
        ctx.raytracing.command_pool,
        ctx.viewport_width,
        ctx.viewport_height,
        hdr_view,
        water_history_desc(depth_view),
    )?;
    let caustic_accum = AccumulationTarget::new(
        ctx.instance,
        ctx.rrdevice,
        ctx.storage,
        ctx.raytracing.command_pool,
        WATER_CAUSTIC_ACCUM_KEY,
        WATER_CAUSTIC_ACCUM_FORMAT,
    )?;

    for image in history.images {
        ctx.pass_image_states.mark_shader_read_only(image);
    }
    ctx.pass_image_states.forget(caustic_accum.image);
    ctx.world
        .insert_resource(WaterRenderTargets::new(history, caustic_accum));
    Ok(true)
}

unsafe fn setup_water(ctx: &mut EffectContext, rrrender: &RRRender) -> Result<()> {
    if !create_water_render_targets(ctx, rrrender.gbuffer_depth_image_view)? {
        log!("HDR buffer not available, skipping water pipeline");
        return Ok(());
    }

    let (Some(water_targets), Some(hdr_color_view)) = (
        ctx.world.get_resource::<WaterRenderTargets>(),
        ctx.hdr_color_view,
    ) else {
        log!("Water buffer not available, skipping water pipeline");
        return Ok(());
    };

    let gpu_state = super::pipeline::create_water_pipeline(
        ctx.instance,
        ctx.rrdevice,
        rrrender,
        ctx.graphics,
        ctx.raytracing,
        &water_targets,
        hdr_color_view,
        MAX_FRAMES_IN_FLIGHT,
    )?;
    drop(water_targets);
    ensure_effect_trace_pipeline(ctx.instance, ctx.rrdevice, ctx.world, MAX_FRAMES_IN_FLIGHT)?;
    let trace_blocks = super::pipeline::water_trace_blocks(ctx.rrdevice, &gpu_state)?;
    ctx.world.insert_resource(trace_blocks);
    ctx.world.insert_resource(gpu_state);

    log!("Water pipeline created successfully");
    Ok(())
}

unsafe fn resize_water_render_targets(ctx: &mut EffectContext) -> Result<()> {
    let depth_view = ctx
        .world
        .resource::<RenderTargets>()
        .render
        .gbuffer_depth_image_view;
    if depth_view == vk::ImageView::null() {
        return Ok(());
    }
    if ctx.world.get_resource::<WaterRenderTargets>().is_none() {
        return Ok(());
    }

    release_water_render_targets(ctx);
    if !create_water_render_targets(ctx, depth_view)? {
        return Ok(());
    }

    update_water_caustic_descriptor(ctx)
}

unsafe fn release_water_render_targets(ctx: &mut EffectContext) {
    let Some(mut targets) = ctx.world.get_resource_mut::<WaterRenderTargets>() else {
        return;
    };
    for image in targets.history.images {
        ctx.pass_image_states.forget(image);
    }
    ctx.pass_image_states.forget(targets.caustic_accum.image);
    targets.destroy_gpu(ctx.rrdevice);
    targets.forget_bindings();
}

unsafe fn update_water_caustic_descriptor(ctx: &mut EffectContext) -> Result<()> {
    let Some(caustic_accum_view) = ctx
        .world
        .get_resource::<WaterRenderTargets>()
        .map(|targets| targets.caustic_accum.view)
    else {
        return Ok(());
    };
    let Some(hdr_color_view) = ctx.hdr_color_view else {
        return Ok(());
    };

    let raytracing = &*ctx.raytracing;
    let Some(mut gpu_state) = ctx.world.get_resource_mut::<WaterGpuState>() else {
        return Ok(());
    };
    let tlas = raytracing
        .acceleration_structure
        .as_ref()
        .and_then(|accel| accel.tlas.acceleration_structure);
    let (Some(position_image_view), Some(scene_buffer), Some(water_ubo)) = (
        raytracing
            .gbuffer
            .as_ref()
            .map(|gbuffer| gbuffer.position_image_view),
        raytracing.scene_uniform_buffer_handle(),
        gpu_state.ubo.as_ref().map(|ubo| ubo.handle()),
    ) else {
        return Ok(());
    };
    let Some(descriptor) = gpu_state.caustic_descriptor.as_mut() else {
        return Ok(());
    };

    descriptor.allocate_and_update(
        ctx.rrdevice,
        caustic_accum_view,
        position_image_view,
        tlas,
        scene_buffer,
        water_ubo,
        hdr_color_view,
    )?;
    gpu_state.caustic_bound_tlas = tlas.unwrap_or_default();
    Ok(())
}

fn water_history_desc(depth_view: vk::ImageView) -> HistoryTargetsDesc {
    HistoryTargetsDesc {
        keys: WATER_HISTORY_KEYS,
        format: HDR_FORMAT,
        usage: vk::ImageUsageFlags::empty(),
        sampler: linear_clamp_sampler_info(),
        history_load_op: vk::AttachmentLoadOp::DONT_CARE,
        depth_view: Some(depth_view),
    }
}

fn linear_clamp_sampler_info() -> vk::SamplerCreateInfo {
    let address_mode = vk::SamplerAddressMode::CLAMP_TO_EDGE;
    vk::SamplerCreateInfo::builder()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
        .address_mode_u(address_mode)
        .address_mode_v(address_mode)
        .address_mode_w(address_mode)
        .border_color(vk::BorderColor::FLOAT_OPAQUE_BLACK)
        .anisotropy_enable(false)
        .max_anisotropy(1.0)
        .build()
}
