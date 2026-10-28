use crate::frame_context::FrameRenderContext;
use crate::types::{CommandBuffer, Extent2D, Image};

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
    crate::renderer::copy_color_image(&ctx.device, cmd, hdr_image, scene_color_image, extent);
}
