use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::LightningRenderTargets;
use crate::ecs::systems::lightning::descriptors::LightningResolveDescriptorSet;
use crate::vulkanr::pipeline::RRPipeline;
use thyllore_vulkan_core::{
    record_overlay_draws, FrameRenderContext, OverlayAttachmentLoad, OverlayDraw, OverlayPass,
};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LightningPushConstants {
    pub mode: i32,
    pub step_count: i32,
    pub debug_view: i32,
}

impl LightningPushConstants {
    pub fn new(mode: i32, step_count: i32, debug_view: i32) -> Self {
        Self {
            mode,
            step_count,
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

pub struct LightningInstanceDraw {
    pub ubo_dynamic_offset: u32,
    pub segments_dynamic_offset: u32,
    pub scissor: vk::Rect2D,
}

fn full_area(extent: vk::Extent2D) -> vk::Rect2D {
    vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent,
    }
}

pub unsafe fn record_lightning_resolve_pass(
    ctx: &FrameRenderContext,
    targets: &LightningRenderTargets,
    pipeline: &RRPipeline,
    descriptor: &LightningResolveDescriptorSet,
    draws: &[LightningInstanceDraw],
    push_constants: LightningPushConstants,
    image_index: usize,
    cmd: vk::CommandBuffer,
) -> Result<()> {
    let device = &ctx.device.device;
    let extent = targets.extent();
    let pass = OverlayPass {
        render_pass: targets.render_pass,
        framebuffer: targets.framebuffer,
        extent,
    };
    let frame_set = ctx.graphics.frame_set.sets[image_index];
    let sets = [frame_set, descriptor.descriptor_set];
    let dynamic_offsets: Vec<[u32; 2]> = draws
        .iter()
        .map(|draw| [draw.ubo_dynamic_offset, draw.segments_dynamic_offset])
        .collect();
    let overlay_draws: Vec<OverlayDraw> = draws
        .iter()
        .zip(&dynamic_offsets)
        .map(|(draw, offsets)| OverlayDraw {
            descriptor_sets: &sets,
            dynamic_offsets: offsets,
            scissor: draw.scissor,
        })
        .collect();
    record_overlay_draws(
        device,
        cmd,
        &pass,
        full_area(extent),
        OverlayAttachmentLoad::Keep,
        pipeline,
        Some(push_constants.as_bytes()),
        &overlay_draws,
    )
}
