use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use thyllore_vulkan_core::render::{create_color_overlay_render_pass, ColorOverlayPassDesc};
use thyllore_vulkan_core::resource::GpuResource;
use thyllore_vulkan_core::types::{Extent2D, HDR_FORMAT};

/// Render pass and framebuffer that blend the lightning resolve pass onto the HDR color image.
#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct LightningRenderTargets {
    pub(crate) render_pass: thyllore_vulkan_core::types::RenderPass,
    pub framebuffer: thyllore_vulkan_core::types::Framebuffer,
    #[gpu_resource(skip)]
    pub width: u32,
    #[gpu_resource(skip)]
    pub height: u32,
}

crate::gpu_resource!(LightningRenderTargets);

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

pub unsafe fn recreate_lightning_render_targets(
    rrdevice: &RRDevice,
    targets: &mut LightningRenderTargets,
    hdr_view: thyllore_vulkan_core::types::ImageView,
    width: u32,
    height: u32,
) -> Result<()> {
    targets.destroy_gpu(rrdevice);
    *targets = create_lightning_render_targets(rrdevice, width, height, hdr_view)?;
    Ok(())
}
