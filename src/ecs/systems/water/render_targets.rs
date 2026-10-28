use anyhow::Result;

use crate::ecs::resource::WaterGpuState;
use crate::ecs::systems::raytracing_systems::ensure_effect_trace_pipeline;
use crate::ecs::{EffectContext, MAX_FRAMES_IN_FLIGHT};
use crate::hooks::effect::EffectHook;
use crate::vulkanr::context::RenderTargets;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::GpuResource;
use thyllore_vulkan_core::effect::water::targets::{
    create_water_gpu_targets, is_depth_view_ready, WaterRenderTargets,
};

pub const WATER_EFFECT_HOOK: EffectHook = EffectHook {
    name: "water",
    setup: Some(setup_water),
    after_overrides: None,
    on_viewport_resize: Some(resize_water_render_targets),
    passes: super::passes::WATER_PASS_NODES,
};

fn insert_water_render_targets(ctx: &mut EffectContext, targets: WaterRenderTargets) {
    for image in targets.history.images() {
        ctx.pass_image_states.mark_shader_read_only(image);
    }
    ctx.pass_image_states.forget(targets.caustic_accum.image());
    ctx.world.insert_resource(targets);
}

unsafe fn setup_water(ctx: &mut EffectContext, rrrender: &RRRender) -> Result<()> {
    let Some(hdr_color_view) = ctx.hdr_color_view else {
        log!("HDR buffer not available, skipping water pipeline");
        return Ok(());
    };
    let targets = create_water_gpu_targets(
        ctx.instance,
        ctx.rrdevice,
        ctx.storage,
        ctx.raytracing.command_pool(),
        ctx.viewport_width,
        ctx.viewport_height,
        hdr_color_view,
        rrrender.gbuffer_depth_image_view(),
    )?;
    insert_water_render_targets(ctx, targets);

    let Some(water_targets) = ctx.world.get_resource::<WaterRenderTargets>() else {
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
        .gbuffer_depth_image_view();
    if !is_depth_view_ready(depth_view) {
        return Ok(());
    }
    if ctx.world.get_resource::<WaterRenderTargets>().is_none() {
        return Ok(());
    }
    let Some(hdr_view) = ctx.hdr_color_view else {
        return Ok(());
    };

    release_water_render_targets(ctx);
    let targets = create_water_gpu_targets(
        ctx.instance,
        ctx.rrdevice,
        ctx.storage,
        ctx.raytracing.command_pool(),
        ctx.viewport_width,
        ctx.viewport_height,
        hdr_view,
        depth_view,
    )?;
    insert_water_render_targets(ctx, targets);

    update_water_caustic_descriptor(ctx)
}

unsafe fn release_water_render_targets(ctx: &mut EffectContext) {
    let Some(mut targets) = ctx.world.get_resource_mut::<WaterRenderTargets>() else {
        return;
    };
    for image in targets.history.images() {
        ctx.pass_image_states.forget(image);
    }
    ctx.pass_image_states.forget(targets.caustic_accum.image());
    targets.destroy_gpu(ctx.rrdevice);
    targets.forget_bindings();
}

unsafe fn update_water_caustic_descriptor(ctx: &mut EffectContext) -> Result<()> {
    let Some(caustic_accum_view) = ctx
        .world
        .get_resource::<WaterRenderTargets>()
        .map(|targets| targets.caustic_accum.view())
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
        .and_then(|accel| accel.tlas.acceleration_structure());
    let (Some(position_image_view), Some(scene_buffer), Some(water_ubo)) = (
        raytracing
            .gbuffer
            .as_ref()
            .map(|gbuffer| gbuffer.position_image_view()),
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
