use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use thyllore_effect_core::{
    WIND_SHADOW_VOLUME_HEIGHT, WIND_SHADOW_VOLUME_RADIAL, WIND_SHADOW_VOLUME_SLOTS,
    WIND_SHADOW_VOLUME_THETA,
};
use thyllore_vulkan_core::render::{create_color_overlay_render_pass, ColorOverlayPassDesc};
use thyllore_vulkan_core::resource::volume_image::VolumeUsage;
use thyllore_vulkan_core::resource::{GpuResource, VolumeImage};
use thyllore_vulkan_core::types::{Extent3D, Format, Instance, HDR_FORMAT};

pub const WIND_SHADOW_VOLUME_FORMAT: Format = Format::R16G16_SFLOAT;

pub fn wind_shadow_volume_extent() -> Extent3D {
    Extent3D {
        width: WIND_SHADOW_VOLUME_RADIAL * WIND_SHADOW_VOLUME_SLOTS,
        height: WIND_SHADOW_VOLUME_HEIGHT,
        depth: WIND_SHADOW_VOLUME_THETA,
    }
}

/// Render pass and framebuffer that blend the wind resolve pass onto the HDR color image,
/// the reduced resolution render pass used by the upsampled resolve path, and the
/// shadow volume baked once per frame for every instance slot.
#[derive(Debug, Default)]
pub struct WindRenderTargets {
    pub(crate) render_pass: thyllore_vulkan_core::types::RenderPass,
    pub framebuffer: thyllore_vulkan_core::types::Framebuffer,
    pub reduced: thyllore_vulkan_core::renderer::ReducedResolveTarget,
    pub shadow_volume: VolumeImage,
    pub width: u32,
    pub height: u32,
}

impl WindRenderTargets {
    pub fn render_pass(&self) -> thyllore_vulkan_core::types::RenderPass {
        self.render_pass
    }

    pub fn extent(&self) -> thyllore_vulkan_core::types::Extent2D {
        thyllore_vulkan_core::types::Extent2D {
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

pub unsafe fn create_wind_render_targets(
    instance: &Instance,
    rrdevice: &RRDevice,
    width: u32,
    height: u32,
    hdr_image_view: thyllore_vulkan_core::types::ImageView,
) -> Result<WindRenderTargets> {
    let render_pass =
        create_color_overlay_render_pass(rrdevice, ColorOverlayPassDesc::preserved(HDR_FORMAT))?;
    let framebuffer = thyllore_vulkan_core::render::create_framebuffer(
        rrdevice,
        render_pass,
        hdr_image_view,
        thyllore_vulkan_core::types::Extent2D { width, height },
    )?;

    let reduced = thyllore_vulkan_core::renderer::ReducedResolveTarget::new(rrdevice, HDR_FORMAT)?;

    let shadow_volume = VolumeImage::new(
        instance,
        rrdevice,
        wind_shadow_volume_extent(),
        WIND_SHADOW_VOLUME_FORMAT,
        VolumeUsage::StorageSampled,
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
        destroy_render_targets(self, rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.framebuffer.is_destroyed()
            && self.render_pass.is_destroyed()
            && self.reduced.is_destroyed()
    }
}

crate::gpu_resource!(WindRenderTargets);

pub unsafe fn destroy_render_targets(targets: &mut WindRenderTargets, rrdevice: &RRDevice) {
    targets.shadow_volume.destroy_gpu(rrdevice);
    targets.reduced.destroy_gpu(rrdevice);
    thyllore_vulkan_core::render::destroy_framebuffer(rrdevice, &mut targets.framebuffer);
    thyllore_vulkan_core::render::destroy_render_pass(rrdevice, &mut targets.render_pass);
}

pub unsafe fn recreate_wind_framebuffer(
    rrdevice: &RRDevice,
    targets: &mut WindRenderTargets,
    hdr_view: thyllore_vulkan_core::types::ImageView,
    width: u32,
    height: u32,
) -> Result<()> {
    thyllore_vulkan_core::render::destroy_framebuffer(rrdevice, &mut targets.framebuffer);
    targets.framebuffer = thyllore_vulkan_core::render::create_framebuffer(
        rrdevice,
        targets.render_pass,
        hdr_view,
        thyllore_vulkan_core::types::Extent2D { width, height },
    )?;
    targets.width = width;
    targets.height = height;
    Ok(())
}
