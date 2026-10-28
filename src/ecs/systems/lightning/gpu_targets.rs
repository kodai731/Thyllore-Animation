use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use thyllore_vulkan_core::render::{create_color_overlay_render_pass, ColorOverlayPassDesc};
use thyllore_vulkan_core::resource::GpuResource;
use thyllore_vulkan_core::types::{Extent2D, HDR_FORMAT};

/// Render pass and framebuffer that blend the lightning resolve pass onto the HDR color image.
#[derive(Debug, Default)]
pub struct LightningRenderTargets {
    pub(crate) render_pass: thyllore_vulkan_core::types::RenderPass,
    pub framebuffer: thyllore_vulkan_core::types::Framebuffer,
    pub width: u32,
    pub height: u32,
}

impl LightningRenderTargets {
    pub fn render_pass(&self) -> thyllore_vulkan_core::types::RenderPass {
        self.render_pass
    }

    pub fn extent(&self) -> Extent2D {
        Extent2D {
            width: self.width,
            height: self.height,
        }
    }

    pub fn overlay_pass(&self) -> thyllore_vulkan_core::renderer::OverlayPass {
        thyllore_vulkan_core::renderer::OverlayPass::new(
            self.render_pass,
            self.framebuffer,
            self.extent(),
            thyllore_vulkan_core::renderer::OverlayAttachmentLoad::Keep,
        )
    }
}

pub unsafe fn create_lightning_render_targets(
    rrdevice: &RRDevice,
    width: u32,
    height: u32,
    hdr_image_view: thyllore_vulkan_core::types::ImageView,
) -> Result<LightningRenderTargets> {
    let render_pass =
        create_color_overlay_render_pass(rrdevice, ColorOverlayPassDesc::preserved(HDR_FORMAT))?;

    let framebuffer = thyllore_vulkan_core::render::create_framebuffer(
        rrdevice,
        render_pass,
        hdr_image_view,
        Extent2D { width, height },
    )?;

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
        destroy_render_targets(self, rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.framebuffer.is_destroyed() && self.render_pass.is_destroyed()
    }
}

pub unsafe fn destroy_render_targets(targets: &mut LightningRenderTargets, rrdevice: &RRDevice) {
    thyllore_vulkan_core::render::destroy_framebuffer(rrdevice, &mut targets.framebuffer);
    thyllore_vulkan_core::render::destroy_render_pass(rrdevice, &mut targets.render_pass);
}

pub unsafe fn recreate_lightning_render_targets(
    rrdevice: &RRDevice,
    targets: &mut LightningRenderTargets,
    hdr_view: thyllore_vulkan_core::types::ImageView,
    width: u32,
    height: u32,
) -> Result<()> {
    destroy_render_targets(targets, rrdevice);
    *targets = create_lightning_render_targets(rrdevice, width, height, hdr_view)?;
    Ok(())
}
