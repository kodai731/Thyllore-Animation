use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::render::{create_color_overlay_render_pass, ColorOverlayPassDesc};
use crate::resource::hdr_buffer::HDR_FORMAT;
use crate::resource::{GpuResource, VolumeImage};
use crate::types::Instance;
use crate::vulkan::*;
use thyllore_effect_core::{
    WIND_SHADOW_VOLUME_HEIGHT, WIND_SHADOW_VOLUME_RADIAL, WIND_SHADOW_VOLUME_SLOTS,
    WIND_SHADOW_VOLUME_THETA,
};

pub const WIND_SHADOW_VOLUME_FORMAT: vk::Format = vk::Format::R16G16_SFLOAT;

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
    Ok(rrdevice.device().create_framebuffer(&info, None)?)
}

/// Render pass and framebuffer that blend the wind resolve pass onto the HDR color image,
/// the reduced resolution render pass used by the upsampled resolve path, and the
/// shadow volume baked once per frame for every instance slot.
#[derive(Debug, Default)]
pub struct WindRenderTargets {
    pub(crate) render_pass: vk::RenderPass,
    pub framebuffer: vk::Framebuffer,
    pub reduced: crate::renderer::ReducedResolveTarget,
    pub shadow_volume: VolumeImage,
    pub width: u32,
    pub height: u32,
}

impl WindRenderTargets {
    pub fn render_pass(&self) -> crate::types::RenderPass {
        crate::types::RenderPass(self.render_pass)
    }

    pub fn extent(&self) -> crate::types::Extent2D {
        crate::types::Extent2D {
            width: self.width,
            height: self.height,
        }
    }

    pub fn overlay_pass(&self) -> crate::renderer::OverlayPass {
        crate::renderer::OverlayPass {
            render_pass: self.render_pass,
            framebuffer: self.framebuffer,
            extent: self.extent().raw(),
            load: crate::renderer::OverlayAttachmentLoad::Keep,
        }
    }
}

pub unsafe fn create_wind_render_targets(
    instance: &Instance,
    rrdevice: &RRDevice,
    width: u32,
    height: u32,
    hdr_image_view: crate::types::ImageView,
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
        hdr_image_view.raw(),
        vk::Extent2D { width, height },
    )?;

    let reduced = crate::renderer::ReducedResolveTarget::new(rrdevice, HDR_FORMAT)?;

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
        reduced,
        shadow_volume,
        width,
        height,
    })
}

impl GpuResource for WindRenderTargets {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        destroy_render_targets(self, &rrdevice.device());
    }

    fn is_destroyed(&self) -> bool {
        self.framebuffer.is_destroyed()
            && self.render_pass.is_destroyed()
            && self.reduced.is_destroyed()
    }
}

pub unsafe fn destroy_render_targets(targets: &mut WindRenderTargets, device: &vulkanalia::Device) {
    targets.shadow_volume.destroy(device);
    targets.reduced.destroy(device);
    if targets.framebuffer != vk::Framebuffer::null() {
        device.destroy_framebuffer(targets.framebuffer, None);
        targets.framebuffer = vk::Framebuffer::null();
    }
    if targets.render_pass != vk::RenderPass::null() {
        device.destroy_render_pass(targets.render_pass, None);
        targets.render_pass = vk::RenderPass::null();
    }
}

pub unsafe fn recreate_wind_framebuffer(
    rrdevice: &RRDevice,
    targets: &mut WindRenderTargets,
    hdr_view: crate::types::ImageView,
    width: u32,
    height: u32,
) -> Result<()> {
    rrdevice
        .device()
        .destroy_framebuffer(targets.framebuffer, None);
    targets.framebuffer = create_framebuffer(
        rrdevice,
        targets.render_pass,
        hdr_view.raw(),
        vk::Extent2D { width, height },
    )?;
    targets.width = width;
    targets.height = height;
    Ok(())
}
