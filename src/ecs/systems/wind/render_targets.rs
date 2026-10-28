use anyhow::Result;

use crate::ecs::resource::WindGpuState;
use crate::ecs::systems::wind::gpu_targets::{
    create_wind_render_targets, recreate_wind_framebuffer, WindRenderTargets,
};
use crate::ecs::{EffectContext, MAX_FRAMES_IN_FLIGHT};
use crate::hooks::effect::EffectHook;
use crate::vulkanr::context::RenderTargets;
use crate::vulkanr::render::RRRender;

pub const WIND_EFFECT_HOOK: EffectHook = EffectHook {
    name: "wind",
    setup: Some(setup_wind),
    after_overrides: None,
    on_viewport_resize: Some(resize_wind_render_targets),
    passes: &[&super::passes::WindPassNode],
};

unsafe fn setup_wind(ctx: &mut EffectContext, rrrender: &RRRender) -> Result<()> {
    let Some(hdr_view) = ctx.hdr_color_view else {
        log!("HDR buffer not available, skipping wind pipeline");
        return Ok(());
    };

    let targets = create_wind_render_targets(
        ctx.instance,
        ctx.rrdevice,
        ctx.viewport_width,
        ctx.viewport_height,
        hdr_view,
    )?;
    let gpu_state = super::pipeline::create_wind_gpu_state(
        ctx.instance,
        ctx.rrdevice,
        rrrender,
        ctx.graphics,
        &targets,
        rrrender.gbuffer_depth_image_view(),
        MAX_FRAMES_IN_FLIGHT,
    )?;
    ctx.world.insert_resource(targets);
    ctx.world.insert_resource(gpu_state);

    log!("Wind pipeline created successfully");
    Ok(())
}

unsafe fn resize_wind_render_targets(ctx: &mut EffectContext) -> Result<()> {
    let Some(hdr_view) = ctx.hdr_color_view else {
        return Ok(());
    };
    let (width, height) = (ctx.viewport_width, ctx.viewport_height);
    let scene_depth_view = ctx
        .world
        .resource::<RenderTargets>()
        .render
        .gbuffer_depth_image_view();

    let Some(mut targets) = ctx.world.get_resource_mut::<WindRenderTargets>() else {
        return Ok(());
    };
    recreate_wind_framebuffer(ctx.rrdevice, &mut targets, hdr_view, width, height)?;

    let Some(mut gpu_state) = ctx.world.get_resource_mut::<WindGpuState>() else {
        return Ok(());
    };
    gpu_state.upsample_bound.forget();
    if let Some(descriptor) = gpu_state.resolve_descriptor.as_ref() {
        descriptor.update_scene_depth(ctx.rrdevice, scene_depth_view)?;
    }
    Ok(())
}
