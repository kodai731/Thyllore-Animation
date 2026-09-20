use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use super::descriptors::RRFlameDescriptorSet;
use crate::vulkanr::pipeline::RRPipeline;
use thyllore_vulkan_core::resource::HistoryTargets;
use thyllore_vulkan_core::{
    record_overlay_draws, FrameRenderContext, OverlayAttachmentLoad, OverlayDraw, OverlayPass,
};

const TRANSPARENT_BLACK: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FlamePushConstants {
    pub mode: i32,
    pub step_count: i32,
    pub debug_view: i32,
}

impl FlamePushConstants {
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

pub unsafe fn record_flame_shading_pass(
    ctx: &FrameRenderContext,
    flame_history: &HistoryTargets,
    pipeline: &RRPipeline,
    descriptor: &RRFlameDescriptorSet,
    history_index: usize,
    ubo_dynamic_offset: u32,
    scissor: vk::Rect2D,
    push_constants: FlamePushConstants,
    image_index: usize,
    cmd: vk::CommandBuffer,
) -> Result<()> {
    let device = &ctx.device.device;

    let extent = flame_history.extent();
    let pass = OverlayPass {
        render_pass: flame_history.render_pass,
        framebuffer: flame_history.framebuffers[history_index],
        extent,
    };
    let render_area = vk::Rect2D {
        offset: vk::Offset2D::default(),
        extent,
    };

    let frame_set = ctx.graphics.frame_set.sets[image_index];
    let overlay_draws = [OverlayDraw {
        descriptor_sets: &[frame_set, descriptor.descriptor_sets[history_index]],
        dynamic_offsets: &[ubo_dynamic_offset],
        scissor,
    }];

    record_overlay_draws(
        device,
        cmd,
        &pass,
        render_area,
        OverlayAttachmentLoad::Clear(&[TRANSPARENT_BLACK; 2]),
        pipeline,
        Some(push_constants.as_bytes()),
        &overlay_draws,
    )
}
