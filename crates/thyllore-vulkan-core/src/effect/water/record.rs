use vulkanalia::prelude::v1_0::*;

use crate::frame_context::FrameRenderContext;
use crate::types::{CommandBuffer, Extent2D, Extent3D, Image, Offset3D};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct WaterPushConstants {
    pub secondary_rays: i32,
    pub debug_view: i32,
}

impl WaterPushConstants {
    pub fn new(secondary_rays: i32, debug_view: i32) -> Self {
        Self {
            secondary_rays,
            debug_view,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self as *const Self) as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

/// Copies the HDR color into the scene color image. The caller brings the source to
/// TRANSFER_SRC_OPTIMAL and the destination to TRANSFER_DST_OPTIMAL.
pub unsafe fn record_water_scene_color_copy(
    ctx: &FrameRenderContext,
    hdr_image: Image,
    scene_color_image: Image,
    extent: Extent2D,
    cmd: CommandBuffer,
) {
    let device = &ctx.device.device();

    let subresource = vk::ImageSubresourceLayers::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .mip_level(0)
        .base_array_layer(0)
        .layer_count(1)
        .build();
    let region = vk::ImageCopy::builder()
        .src_subresource(subresource)
        .src_offset(Offset3D { x: 0, y: 0, z: 0 })
        .dst_subresource(subresource)
        .dst_offset(Offset3D { x: 0, y: 0, z: 0 })
        .extent(Extent3D {
            width: extent.width,
            height: extent.height,
            depth: 1,
        })
        .build();
    device.cmd_copy_image(
        cmd.raw(),
        hdr_image.raw(),
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        scene_color_image.raw(),
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &[region],
    );
}
