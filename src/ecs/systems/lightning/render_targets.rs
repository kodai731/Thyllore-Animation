use anyhow::Result;

use crate::ecs::resource::LightningGpuState;
use crate::ecs::EffectContext;
use crate::hooks::effect::EffectHook;
use crate::vulkanr::context::RenderTargets;
use crate::vulkanr::render::RRRender;
use thyllore_vulkan_core::effect::lightning::targets::LightningRenderTargets;
use thyllore_vulkan_core::effect::lightning::targets::{
    create_lightning_render_targets, recreate_lightning_render_targets,
};

pub const LIGHTNING_EFFECT_HOOK: EffectHook = EffectHook {
    name: "lightning",
    setup: Some(setup_lightning),
    after_overrides: None,
    on_viewport_resize: Some(resize_lightning_render_targets),
    passes: &[&super::passes::LightningPassNode],
};

unsafe fn setup_lightning(ctx: &mut EffectContext, rrrender: &RRRender) -> Result<()> {
    let Some(hdr_view) = ctx.hdr_color_view else {
        log!("HDR buffer not available, skipping lightning pipeline");
        return Ok(());
    };

    let targets = create_lightning_render_targets(
        ctx.rrdevice,
        ctx.viewport_width,
        ctx.viewport_height,
        hdr_view,
    )?;
    let gpu_state = super::pipeline::create_lightning_gpu_state(
        ctx.instance,
        ctx.rrdevice,
        rrrender,
        ctx.graphics,
        &targets,
        rrrender.gbuffer_depth_image_view(),
    )?;
    ctx.world.insert_resource(targets);
    ctx.world.insert_resource(gpu_state);

    log!("Lightning pipeline created successfully");
    Ok(())
}

unsafe fn resize_lightning_render_targets(ctx: &mut EffectContext) -> Result<()> {
    let Some(hdr_view) = ctx.hdr_color_view else {
        return Ok(());
    };
    let (width, height) = (ctx.viewport_width, ctx.viewport_height);
    let scene_depth_view = ctx
        .world
        .resource::<RenderTargets>()
        .render
        .gbuffer_depth_image_view();

    let Some(mut targets) = ctx.world.get_resource_mut::<LightningRenderTargets>() else {
        return Ok(());
    };
    recreate_lightning_render_targets(ctx.rrdevice, &mut targets, hdr_view, width, height)?;

    let Some(gpu_state) = ctx.world.get_resource::<LightningGpuState>() else {
        return Ok(());
    };
    if let Some(descriptor) = gpu_state.resolve_descriptor.as_ref() {
        descriptor.update_scene_depth(ctx.rrdevice, scene_depth_view)?;
    }
    Ok(())
}
