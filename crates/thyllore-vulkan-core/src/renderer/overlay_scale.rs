use crate::core::RRDevice;
use crate::descriptor::OVERLAY_UPSAMPLE;
use crate::render::{create_color_overlay_render_pass, ColorOverlayPassDesc};
use crate::renderer::overlay::{
    OverlayAttachmentLoad, OverlayBlend, OverlayNodeSpec, OverlayPass, OverlayPushConstants,
};
use crate::resource::gpu_resource::GpuResource;
use crate::resource::render_target_transient::TransientDesc;
use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

/// The resolution an overlay is resolved at before the upsample composites it onto the full one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayResolveScale {
    Half,
    Quarter,
}

impl OverlayResolveScale {
    pub fn divisor(self) -> u32 {
        match self {
            OverlayResolveScale::Half => 2,
            OverlayResolveScale::Quarter => 4,
        }
    }

    pub fn reduce_extent(self, full: vk::Extent2D) -> vk::Extent2D {
        let divisor = self.divisor();
        vk::Extent2D {
            width: (full.width / divisor).max(1),
            height: (full.height / divisor).max(1),
        }
    }

    /// Reduced resolution scissor grown by one texel so the upsample taps stay inside resolved pixels.
    pub fn reduce_scissor(self, scissor: vk::Rect2D, reduced_extent: vk::Extent2D) -> vk::Rect2D {
        let divisor = self.divisor() as i32;
        let left = (scissor.offset.x / divisor - 1).max(0);
        let top = (scissor.offset.y / divisor - 1).max(0);
        let right = ((scissor.offset.x + scissor.extent.width as i32).div_euclid(divisor) + 2)
            .min(reduced_extent.width as i32);
        let bottom = ((scissor.offset.y + scissor.extent.height as i32).div_euclid(divisor) + 2)
            .min(reduced_extent.height as i32);

        vk::Rect2D {
            offset: vk::Offset2D { x: left, y: top },
            extent: vk::Extent2D {
                width: (right - left).max(0) as u32,
                height: (bottom - top).max(0) as u32,
            },
        }
    }
}

/// The screen rectangle covering every scissor, grown by two pixels; the full extent when empty.
pub fn union_scissor(
    scissors: impl Iterator<Item = vk::Rect2D>,
    full_extent: vk::Extent2D,
) -> vk::Rect2D {
    let mut bounds: Option<(i32, i32, i32, i32)> = None;
    for scissor in scissors {
        let candidate = (
            scissor.offset.x,
            scissor.offset.y,
            scissor.offset.x + scissor.extent.width as i32,
            scissor.offset.y + scissor.extent.height as i32,
        );
        bounds = Some(match bounds {
            Some(current) => (
                current.0.min(candidate.0),
                current.1.min(candidate.1),
                current.2.max(candidate.2),
                current.3.max(candidate.3),
            ),
            None => candidate,
        });
    }

    let Some((left, top, right, bottom)) = bounds else {
        return vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: full_extent,
        };
    };
    let left = (left - 2).max(0);
    let top = (top - 2).max(0);
    let right = (right + 2).min(full_extent.width as i32);
    let bottom = (bottom + 2).min(full_extent.height as i32);

    vk::Rect2D {
        offset: vk::Offset2D { x: left, y: top },
        extent: vk::Extent2D {
            width: (right - left).max(0) as u32,
            height: (bottom - top).max(0) as u32,
        },
    }
}

/// Composites a reduced resolution overlay onto the full resolution target with a depth-aware
/// bilinear filter (`shaders/overlay/upsampleFragment.slang`).
pub const UPSAMPLE_OVERLAY: OverlayNodeSpec = OverlayNodeSpec {
    shaders: &OVERLAY_UPSAMPLE,
    blends: &[OverlayBlend::Premultiplied],
    depth_test: None,
    push_constants: OverlayPushConstants::None,
};

/// The render pass of a reduced resolution overlay: a transient color image cleared to transparent
/// and left readable for the upsample. The scale is chosen per frame, so only the format is fixed.
#[derive(Clone, Debug, Default)]
pub struct ReducedResolveTarget {
    pub render_pass: vk::RenderPass,
    pub format: vk::Format,
}

impl ReducedResolveTarget {
    pub unsafe fn new(rrdevice: &RRDevice, format: vk::Format) -> Result<Self> {
        let render_pass = create_color_overlay_render_pass(
            rrdevice,
            ColorOverlayPassDesc {
                format,
                load_op: vk::AttachmentLoadOp::CLEAR,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
        )?;
        Ok(Self {
            render_pass,
            format,
        })
    }

    pub fn transient_desc(&self, scale: OverlayResolveScale, full: vk::Extent2D) -> TransientDesc {
        let extent = scale.reduce_extent(full);
        TransientDesc {
            width: extent.width,
            height: extent.height,
            format: self.format,
            usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
        }
    }

    pub fn overlay_pass(
        &self,
        framebuffer: vk::Framebuffer,
        scale: OverlayResolveScale,
        full: vk::Extent2D,
    ) -> OverlayPass {
        OverlayPass {
            render_pass: self.render_pass,
            framebuffer,
            extent: scale.reduce_extent(full),
            load: OverlayAttachmentLoad::Clear([0.0; 4]),
        }
    }

    pub unsafe fn destroy(&mut self, device: &Device) {
        if self.render_pass != vk::RenderPass::null() {
            device.destroy_render_pass(self.render_pass, None);
            self.render_pass = vk::RenderPass::null();
        }
    }
}

impl GpuResource for ReducedResolveTarget {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, width: u32, height: u32) -> vk::Rect2D {
        vk::Rect2D {
            offset: vk::Offset2D { x, y },
            extent: vk::Extent2D { width, height },
        }
    }

    #[test]
    fn reduce_extent_never_reaches_zero() {
        let tiny = vk::Extent2D {
            width: 3,
            height: 1,
        };
        assert_eq!(
            OverlayResolveScale::Quarter.reduce_extent(tiny),
            vk::Extent2D {
                width: 1,
                height: 1
            }
        );
    }

    #[test]
    fn reduced_scissor_grows_by_one_texel_inside_the_reduced_extent() {
        let reduced = vk::Extent2D {
            width: 100,
            height: 50,
        };
        let scissor = OverlayResolveScale::Quarter.reduce_scissor(rect(40, 20, 80, 40), reduced);
        assert_eq!(scissor, rect(9, 4, 23, 13));

        let clamped = OverlayResolveScale::Half.reduce_scissor(rect(0, 0, 400, 200), reduced);
        assert_eq!(clamped, rect(0, 0, 100, 50));
    }

    #[test]
    fn union_scissor_covers_every_rect_and_falls_back_to_the_full_extent() {
        let full = vk::Extent2D {
            width: 200,
            height: 100,
        };
        let union = union_scissor(
            [rect(10, 10, 20, 20), rect(50, 40, 30, 30)].into_iter(),
            full,
        );
        assert_eq!(union, rect(8, 8, 74, 64));
        assert_eq!(
            union_scissor(std::iter::empty(), full),
            rect(0, 0, 200, 100)
        );
    }
}
