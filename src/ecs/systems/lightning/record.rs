use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::LightningRenderTargets;
use crate::ecs::systems::lightning::descriptors::LightningResolveDescriptorSet;
use crate::vulkanr::pipeline::RRPipeline;
use thyllore_vulkan_core::{
    begin_overlay_render_pass, draw_fullscreen_triangle, set_full_viewport, FrameRenderContext,
    OverlayAttachmentLoad,
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
    begin_overlay_render_pass(
        device,
        cmd,
        targets.render_pass,
        targets.framebuffer,
        vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        },
        OverlayAttachmentLoad::Keep,
    );

    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
    set_full_viewport(device, cmd, extent);
    device.cmd_push_constants(
        cmd,
        pipeline.pipeline_layout,
        vk::ShaderStageFlags::FRAGMENT,
        0,
        push_constants.as_bytes(),
    );

    let frame_set = ctx.graphics.frame_set.sets[image_index];
    for draw in draws {
        device.cmd_set_scissor(cmd, 0, &[draw.scissor]);
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline_layout,
            0,
            &[frame_set, descriptor.descriptor_set],
            &[draw.ubo_dynamic_offset, draw.segments_dynamic_offset],
        );
        draw_fullscreen_triangle(device, cmd);
    }

    device.cmd_end_render_pass(cmd);
    Ok(())
}
