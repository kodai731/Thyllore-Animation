use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use super::descriptors::RRFlameDescriptorSet;
use crate::vulkanr::pipeline::RRPipeline;
use thyllore_vulkan_core::resource::HistoryTargets;
use thyllore_vulkan_core::FrameRenderContext;

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

    let render_area = vk::Rect2D::builder()
        .offset(vk::Offset2D::default())
        .extent(flame_history.extent());

    let clear_values = [
        vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        },
        vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        },
    ];

    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(flame_history.render_pass)
        .framebuffer(flame_history.framebuffers[history_index])
        .render_area(render_area)
        .clear_values(&clear_values);

    device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);

    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);

    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(flame_history.width as f32)
        .height(flame_history.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);
    device.cmd_set_viewport(cmd, 0, &[viewport]);
    device.cmd_set_scissor(cmd, 0, &[scissor]);

    let frame_set = ctx.graphics.frame_set.sets[image_index];
    device.cmd_bind_descriptor_sets(
        cmd,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline_layout,
        0,
        &[frame_set, descriptor.descriptor_sets[history_index]],
        &[ubo_dynamic_offset],
    );

    device.cmd_push_constants(
        cmd,
        pipeline.pipeline_layout,
        vk::ShaderStageFlags::FRAGMENT,
        0,
        push_constants.as_bytes(),
    );

    device.cmd_draw(cmd, 3, 1, 0, 0);

    device.cmd_end_render_pass(cmd);
    Ok(())
}
