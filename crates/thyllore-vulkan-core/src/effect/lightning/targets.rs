use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::render::{create_color_overlay_render_pass, ColorOverlayPassDesc};
use crate::resource::GpuResource;
use crate::types::HDR_FORMAT;
use crate::vulkan::*;

/// Render pass and framebuffer that blend the lightning resolve pass onto the HDR color image.
#[derive(Debug, Default)]
pub struct LightningRenderTargets {
    pub(crate) render_pass: vk::RenderPass,
    pub framebuffer: vk::Framebuffer,
    pub width: u32,
    pub height: u32,
}

impl LightningRenderTargets {
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

pub unsafe fn create_lightning_render_targets(
    rrdevice: &RRDevice,
    width: u32,
    height: u32,
    hdr_image_view: crate::types::ImageView,
) -> Result<LightningRenderTargets> {
    let render_pass =
        create_color_overlay_render_pass(rrdevice, ColorOverlayPassDesc::preserved(HDR_FORMAT))?
            .raw();

    let attachments = [hdr_image_view.raw()];
    let framebuffer_info = vk::FramebufferCreateInfo::builder()
        .render_pass(render_pass)
        .attachments(&attachments)
        .width(width)
        .height(height)
        .layers(1);
    let framebuffer = rrdevice
        .device()
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
        destroy_render_targets(self, &rrdevice.device());
    }

    fn is_destroyed(&self) -> bool {
        self.framebuffer.is_destroyed() && self.render_pass.is_destroyed()
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

pub unsafe fn recreate_lightning_render_targets(
    rrdevice: &RRDevice,
    targets: &mut LightningRenderTargets,
    hdr_view: crate::types::ImageView,
    width: u32,
    height: u32,
) -> Result<()> {
    destroy_render_targets(targets, &rrdevice.device());
    *targets = create_lightning_render_targets(rrdevice, width, height, hdr_view)?;
    Ok(())
}
