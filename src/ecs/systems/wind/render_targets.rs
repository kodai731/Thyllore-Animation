use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::{WindGpuState, WindRenderTargets};
use crate::ecs::{EffectContext, MAX_FRAMES_IN_FLIGHT};
use crate::hooks::effect::EffectHook;
use crate::vulkanr::context::RenderTargets;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::render::{create_color_overlay_render_pass, ColorOverlayPassDesc, RRRender};
use crate::vulkanr::resource::hdr_buffer::HDR_FORMAT;
use crate::vulkanr::resource::{GpuResource, VolumeImage};
use thyllore_effect_core::{
    WIND_SHADOW_VOLUME_HEIGHT, WIND_SHADOW_VOLUME_RADIAL, WIND_SHADOW_VOLUME_SLOTS,
    WIND_SHADOW_VOLUME_THETA,
};

pub const WIND_SHADOW_VOLUME_FORMAT: vk::Format = vk::Format::R16G16_SFLOAT;

pub const WIND_EFFECT_HOOK: EffectHook = EffectHook {
    name: "wind",
    setup: Some(setup_wind),
    after_overrides: None,
    on_viewport_resize: Some(resize_wind_render_targets),
    passes: &[&super::passes::WindPassNode],
};

pub fn wind_shadow_volume_extent() -> vk::Extent3D {
    vk::Extent3D {
        width: WIND_SHADOW_VOLUME_RADIAL * WIND_SHADOW_VOLUME_SLOTS,
        height: WIND_SHADOW_VOLUME_HEIGHT,
        depth: WIND_SHADOW_VOLUME_THETA,
    }
}

unsafe fn create_framebuffer(
    rrdevice: &RRDevice,
    render_pass: vk::RenderPass,
    attachment: vk::ImageView,
    extent: vk::Extent2D,
) -> Result<vk::Framebuffer> {
    let attachments = [attachment];
    let info = vk::FramebufferCreateInfo::builder()
        .render_pass(render_pass)
        .attachments(&attachments)
        .width(extent.width)
        .height(extent.height)
        .layers(1);
    Ok(rrdevice.device.create_framebuffer(&info, None)?)
}

pub unsafe fn create_wind_render_targets(
    instance: &Instance,
    rrdevice: &RRDevice,
    width: u32,
    height: u32,
    hdr_image_view: vk::ImageView,
) -> Result<WindRenderTargets> {
    let render_pass = create_color_overlay_render_pass(
        rrdevice,
        ColorOverlayPassDesc {
            format: HDR_FORMAT,
            load_op: vk::AttachmentLoadOp::LOAD,
            initial_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        },
    )?;
    let framebuffer = create_framebuffer(
        rrdevice,
        render_pass,
        hdr_image_view,
        vk::Extent2D { width, height },
    )?;

    let half_render_pass = create_color_overlay_render_pass(
        rrdevice,
        ColorOverlayPassDesc {
            format: HDR_FORMAT,
            load_op: vk::AttachmentLoadOp::CLEAR,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        },
    )?;

    let shadow_volume = VolumeImage::new(
        instance,
        rrdevice,
        wind_shadow_volume_extent(),
        WIND_SHADOW_VOLUME_FORMAT,
        vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
    )?;

    log!("Created wind render targets: {}x{}", width, height);
    Ok(WindRenderTargets {
        render_pass,
        framebuffer,
        half_render_pass,
        shadow_volume,
        width,
        height,
    })
}

impl GpuResource for WindRenderTargets {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        destroy_render_targets(self, &rrdevice.device);
    }
}

pub unsafe fn destroy_render_targets(targets: &mut WindRenderTargets, device: &vulkanalia::Device) {
    targets.shadow_volume.destroy(device);
    if targets.half_render_pass != vk::RenderPass::null() {
        device.destroy_render_pass(targets.half_render_pass, None);
        targets.half_render_pass = vk::RenderPass::null();
    }
    if targets.framebuffer != vk::Framebuffer::null() {
        device.destroy_framebuffer(targets.framebuffer, None);
        targets.framebuffer = vk::Framebuffer::null();
    }
    if targets.render_pass != vk::RenderPass::null() {
        device.destroy_render_pass(targets.render_pass, None);
        targets.render_pass = vk::RenderPass::null();
    }
}

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
        rrrender.gbuffer_depth_image_view,
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
        .gbuffer_depth_image_view;

    let Some(mut targets) = ctx.world.get_resource_mut::<WindRenderTargets>() else {
        return Ok(());
    };
    destroy_render_targets(&mut targets, &ctx.rrdevice.device);
    *targets = create_wind_render_targets(ctx.instance, ctx.rrdevice, width, height, hdr_view)?;

    let Some(mut gpu_state) = ctx.world.get_resource_mut::<WindGpuState>() else {
        return Ok(());
    };
    gpu_state.upsample_bound.forget();
    if let Some(descriptor) = gpu_state.resolve_descriptor.as_ref() {
        descriptor.update_scene_depth(ctx.rrdevice, scene_depth_view)?;
        descriptor.update_shadow_volume(ctx.rrdevice, &targets.shadow_volume)?;
    }
    if let Some(descriptor) = gpu_state.shadow_bake_descriptor.as_ref() {
        descriptor.update_shadow_volume(ctx.rrdevice, &targets.shadow_volume)?;
    }
    Ok(())
}
