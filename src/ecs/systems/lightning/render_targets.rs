use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::{LightningGpuState, LightningRenderTargets};
use crate::ecs::EffectContext;
use crate::hooks::effect::EffectHook;
use crate::vulkanr::context::RenderTargets;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::render::{create_color_overlay_render_pass, ColorOverlayPassDesc, RRRender};
use crate::vulkanr::resource::hdr_buffer::HDR_FORMAT;
use crate::vulkanr::resource::GpuResource;

pub const LIGHTNING_EFFECT_HOOK: EffectHook = EffectHook {
    name: "lightning",
    setup: Some(setup_lightning),
    after_overrides: None,
    on_viewport_resize: Some(resize_lightning_render_targets),
    passes: &[&super::passes::LightningPassNode],
};

pub unsafe fn create_lightning_render_targets(
    rrdevice: &RRDevice,
    width: u32,
    height: u32,
    hdr_image_view: vk::ImageView,
) -> Result<LightningRenderTargets> {
    let render_pass = create_color_overlay_render_pass(
        rrdevice,
        ColorOverlayPassDesc {
            format: HDR_FORMAT,
            load_op: vk::AttachmentLoadOp::LOAD,
            initial_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        },
    )?;

    let attachments = [hdr_image_view];
    let framebuffer_info = vk::FramebufferCreateInfo::builder()
        .render_pass(render_pass)
        .attachments(&attachments)
        .width(width)
        .height(height)
        .layers(1);
    let framebuffer = rrdevice
        .device
        .create_framebuffer(&framebuffer_info, None)?;

    log!("Created lightning render targets: {}x{}", width, height);
    Ok(LightningRenderTargets {
        render_pass,
        framebuffer,
        width,
        height,
    })
}

impl GpuResource for LightningRenderTargets {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        destroy_render_targets(self, &rrdevice.device);
    }
}

unsafe fn destroy_render_targets(
    targets: &mut LightningRenderTargets,
    device: &vulkanalia::Device,
) {
    if targets.framebuffer != vk::Framebuffer::null() {
        device.destroy_framebuffer(targets.framebuffer, None);
        targets.framebuffer = vk::Framebuffer::null();
    }
    if targets.render_pass != vk::RenderPass::null() {
        device.destroy_render_pass(targets.render_pass, None);
        targets.render_pass = vk::RenderPass::null();
    }
}

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
        rrrender.gbuffer_depth_image_view,
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
        .gbuffer_depth_image_view;

    let Some(mut targets) = ctx.world.get_resource_mut::<LightningRenderTargets>() else {
        return Ok(());
    };
    destroy_render_targets(&mut targets, &ctx.rrdevice.device);
    *targets = create_lightning_render_targets(ctx.rrdevice, width, height, hdr_view)?;

    let Some(gpu_state) = ctx.world.get_resource::<LightningGpuState>() else {
        return Ok(());
    };
    if let Some(descriptor) = gpu_state.resolve_descriptor.as_ref() {
        descriptor.update_scene_depth(ctx.rrdevice, scene_depth_view)?;
    }
    Ok(())
}
