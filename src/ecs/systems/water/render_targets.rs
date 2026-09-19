use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::WaterRenderTargets;
use crate::ecs::{EffectContext, MAX_FRAMES_IN_FLIGHT};
use crate::hooks::effect::EffectHook;
use crate::vulkanr::context::RenderTargets;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{GpuResource, WaterBuffer};

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

    let buffer = WaterBuffer::new(
        ctx.instance,
        ctx.rrdevice,
        ctx.storage,
        ctx.raytracing.command_pool,
        ctx.viewport_width,
        ctx.viewport_height,
        hdr_view,
        depth_view,
    )?;

    for image in buffer.history_images {
        ctx.pass_image_states.mark_shader_read_only(image);
    }
    ctx.pass_image_states.forget(buffer.caustic_accum_image);
    ctx.world.insert_resource(WaterRenderTargets::new(buffer));
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

    super::pipeline::create_water_pipeline(
        ctx.instance,
        ctx.rrdevice,
        rrrender,
        ctx.graphics,
        ctx.raytracing,
        &water_targets.buffer,
        hdr_color_view,
        MAX_FRAMES_IN_FLIGHT,
    )?;
    drop(water_targets);
    let trace_blocks = super::pipeline::water_trace_blocks(ctx.rrdevice, ctx.raytracing)?;
    ctx.world.insert_resource(trace_blocks);

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
    for image in targets.buffer.history_images {
        ctx.pass_image_states.forget(image);
    }
    ctx.pass_image_states
        .forget(targets.buffer.caustic_accum_image);
    targets.destroy_gpu(ctx.rrdevice);
    targets.forget_bindings();
}

unsafe fn update_water_caustic_descriptor(ctx: &mut EffectContext) -> Result<()> {
    let Some(caustic_accum_view) = ctx
        .world
        .get_resource::<WaterRenderTargets>()
        .map(|targets| targets.buffer.caustic_accum_view)
    else {
        return Ok(());
    };
    let Some(hdr_color_view) = ctx.hdr_color_view else {
        return Ok(());
    };

    let rrdevice = ctx.rrdevice;
    let raytracing = &mut *ctx.raytracing;
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
        raytracing.water_ubo.as_ref().map(|ubo| ubo.handle()),
    ) else {
        return Ok(());
    };
    let Some(descriptor) = raytracing.water_caustic_descriptor.as_mut() else {
        return Ok(());
    };

    descriptor.allocate_and_update(
        rrdevice,
        caustic_accum_view,
        position_image_view,
        tlas,
        scene_buffer,
        water_ubo,
        hdr_color_view,
    )
}
