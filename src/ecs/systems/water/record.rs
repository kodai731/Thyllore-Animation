use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use super::descriptors::RRWaterDescriptorSet;
use crate::vulkanr::pipeline::RRPipeline;
use thyllore_vulkan_core::frame_context::FrameRenderContext;
use thyllore_vulkan_core::resource::HistoryTargets;

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
    hdr_image: vk::Image,
    scene_color_image: vk::Image,
    extent: vk::Extent2D,
    cmd: vk::CommandBuffer,
) {
    let device = &ctx.device.device;

    let subresource = vk::ImageSubresourceLayers::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .mip_level(0)
        .base_array_layer(0)
        .layer_count(1)
        .build();
    let region = vk::ImageCopy::builder()
        .src_subresource(subresource)
        .src_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .dst_subresource(subresource)
        .dst_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .extent(vk::Extent3D {
            width: extent.width,
            height: extent.height,
            depth: 1,
        })
        .build();
    device.cmd_copy_image(
        cmd,
        hdr_image,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        scene_color_image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &[region],
    );
}

pub unsafe fn record_water_shading_pass(
    ctx: &FrameRenderContext,
    water_history: &HistoryTargets,
    pipeline: &RRPipeline,
    descriptor: &RRWaterDescriptorSet,
    ubo_dynamic_offset: u32,
    scissor: vk::Rect2D,
    push_constants: WaterPushConstants,
    image_index: usize,
    frame_slot: usize,
    history_index: usize,
    cmd: vk::CommandBuffer,
) -> Result<()> {
    let device = &ctx.device.device;

    let render_area = scissor;

    let clear_values: [vk::ClearValue; 0] = [];

    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(water_history.render_pass)
        .framebuffer(water_history.framebuffers[history_index])
        .render_area(render_area)
        .clear_values(&clear_values);

    device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);

    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);

    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(water_history.width as f32)
        .height(water_history.height as f32)
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
        &[
            frame_set,
            descriptor.descriptor_set(frame_slot, history_index)?,
        ],
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
