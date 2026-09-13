use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::app::{App, AppData};
use crate::ecs::resource::{half_extent, WindGpuState, WindRenderTargets};
use crate::hooks::effect::EffectHook;
use crate::vulkanr::context::RenderTargets;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::image::{create_image, create_image_view};
use crate::vulkanr::render::{create_color_overlay_render_pass, ColorOverlayPassDesc, RRRender};
use crate::vulkanr::resource::hdr_buffer::HDR_FORMAT;
use crate::vulkanr::resource::VolumeImage;
use thyllore_effect_core::{
    WIND_SHADOW_VOLUME_HEIGHT, WIND_SHADOW_VOLUME_RADIAL, WIND_SHADOW_VOLUME_SLOTS,
    WIND_SHADOW_VOLUME_THETA,
};

pub const WIND_SHADOW_VOLUME_FORMAT: vk::Format = vk::Format::R16G16_SFLOAT;

pub const WIND_EFFECT_HOOK: EffectHook = EffectHook {
    name: "wind",
    setup: Some(setup_wind),
    on_viewport_resize: Some(resize_wind_render_targets),
    destroy: Some(destroy_wind_render_targets),
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

    let half = half_extent(width, height);
    let (half_color_image, half_color_image_memory) = create_image(
        instance,
        rrdevice,
        half.width,
        half.height,
        1,
        vk::SampleCountFlags::_1,
        HDR_FORMAT,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    let half_color_image_view = create_image_view(
        rrdevice,
        half_color_image,
        HDR_FORMAT,
        vk::ImageAspectFlags::COLOR,
        1,
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
    let half_framebuffer =
        create_framebuffer(rrdevice, half_render_pass, half_color_image_view, half)?;

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
        half_framebuffer,
        half_color_image,
        half_color_image_memory,
        half_color_image_view,
        shadow_volume,
        width,
        height,
    })
}

pub unsafe fn destroy_render_targets(targets: &mut WindRenderTargets, device: &vulkanalia::Device) {
    targets.shadow_volume.destroy(device);
    if targets.half_framebuffer != vk::Framebuffer::null() {
        device.destroy_framebuffer(targets.half_framebuffer, None);
        targets.half_framebuffer = vk::Framebuffer::null();
    }
    if targets.half_render_pass != vk::RenderPass::null() {
        device.destroy_render_pass(targets.half_render_pass, None);
        targets.half_render_pass = vk::RenderPass::null();
    }
    if targets.half_color_image_view != vk::ImageView::null() {
        device.destroy_image_view(targets.half_color_image_view, None);
        targets.half_color_image_view = vk::ImageView::null();
    }
    if targets.half_color_image != vk::Image::null() {
        device.destroy_image(targets.half_color_image, None);
        targets.half_color_image = vk::Image::null();
    }
    if targets.half_color_image_memory != vk::DeviceMemory::null() {
        device.free_memory(targets.half_color_image_memory, None);
        targets.half_color_image_memory = vk::DeviceMemory::null();
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

unsafe fn destroy_gpu_state(state: &mut WindGpuState, device: &vulkanalia::Device) {
    if let Some(mut descriptor) = state.resolve_descriptor.take() {
        descriptor.destroy(device);
    }
    if let Some(pipeline) = state.resolve_pipeline.take() {
        pipeline.destroy(device);
    }
    if let Some(mut ubo) = state.ubo.take() {
        ubo.destroy(device);
    }
    if let Some(mut descriptor) = state.upsample_descriptor.take() {
        descriptor.destroy(device);
    }
    if let Some(pipeline) = state.upsample_pipeline.take() {
        pipeline.destroy(device);
    }
    if let Some(mut descriptor) = state.shadow_bake_descriptor.take() {
        descriptor.destroy(device);
    }
    if let Some(pipeline) = state.shadow_bake_pipeline.take() {
        pipeline.destroy(device);
    }
}

unsafe fn setup_wind(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    rrrender: &RRRender,
) -> Result<()> {
    let Some(hdr_view) = data
        .viewport
        .hdr_buffer
        .as_ref()
        .map(|hdr| hdr.color_image_view)
    else {
        log!("HDR buffer not available, skipping wind pipeline");
        return Ok(());
    };

    let targets = create_wind_render_targets(
        instance,
        rrdevice,
        data.viewport.width,
        data.viewport.height,
        hdr_view,
    )?;
    let gpu_state = super::pipeline::create_wind_gpu_state(
        instance,
        rrdevice,
        rrrender,
        &data.graphics_resources,
        &targets,
        rrrender.gbuffer_depth_image_view,
    )?;
    data.ecs_world.insert_resource(targets);
    data.ecs_world.insert_resource(gpu_state);

    log!("Wind pipeline created successfully");
    Ok(())
}

unsafe fn resize_wind_render_targets(app: &mut App) -> Result<()> {
    let Some(hdr_view) = app
        .data
        .viewport
        .hdr_buffer
        .as_ref()
        .map(|hdr| hdr.color_image_view)
    else {
        return Ok(());
    };
    let (width, height) = (app.data.viewport.width, app.data.viewport.height);
    let scene_depth_view = app
        .resource::<RenderTargets>()
        .render
        .gbuffer_depth_image_view;

    let Some(mut targets) = app.data.ecs_world.get_resource_mut::<WindRenderTargets>() else {
        return Ok(());
    };
    destroy_render_targets(&mut targets, &app.rrdevice.device);
    *targets = create_wind_render_targets(&app.instance, &app.rrdevice, width, height, hdr_view)?;

    let Some(gpu_state) = app.data.ecs_world.get_resource::<WindGpuState>() else {
        return Ok(());
    };
    if let Some(descriptor) = gpu_state.resolve_descriptor.as_ref() {
        descriptor.update_scene_depth(&app.rrdevice, scene_depth_view)?;
        descriptor.update_shadow_volume(&app.rrdevice, &targets.shadow_volume)?;
    }
    if let Some(descriptor) = gpu_state.shadow_bake_descriptor.as_ref() {
        descriptor.update_shadow_volume(&app.rrdevice, &targets.shadow_volume)?;
    }
    if let Some(descriptor) = gpu_state.upsample_descriptor.as_ref() {
        descriptor.update_image_views(
            &app.rrdevice,
            targets.half_color_image_view,
            scene_depth_view,
        )?;
    }
    Ok(())
}

unsafe fn destroy_wind_render_targets(app: &mut App) -> Result<()> {
    if let Some(mut gpu_state) = app.data.ecs_world.get_resource_mut::<WindGpuState>() {
        destroy_gpu_state(&mut gpu_state, &app.rrdevice.device);
    }
    if let Some(mut targets) = app.data.ecs_world.get_resource_mut::<WindRenderTargets>() {
        destroy_render_targets(&mut targets, &app.rrdevice.device);
    }
    Ok(())
}
