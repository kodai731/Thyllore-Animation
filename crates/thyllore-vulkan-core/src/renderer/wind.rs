use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::descriptor::{
    RRWindDescriptorSet, RRWindShadowBakeDescriptorSet, RRWindUpsampleDescriptorSet,
};
use crate::frame_context::FrameRenderContext;
use crate::pipeline::RRPipeline;
use crate::renderer::push_constants::WindPushConstants;
use crate::resource::wind_buffer::{wind_shadow_volume_extent, WindBuffer};
use thyllore_effect_core::WIND_SHADOW_VOLUME_SLOTS;

/// Must match local_size in shadowBake.comp.
const SHADOW_BAKE_WORKGROUP_SIZE: u32 = 8;

pub struct WindInstanceDraw {
    pub ubo_dynamic_offset: u32,
    pub scissor: vk::Rect2D,
}

fn shadow_volume_subresource() -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    }
}

/// The volume is rewritten in full every frame, so the previous contents are discarded.
unsafe fn insert_pre_bake_barrier(
    device: &Device,
    wind_buffer: &WindBuffer,
    cmd: vk::CommandBuffer,
) {
    let barrier = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::SHADER_READ)
        .dst_access_mask(vk::AccessFlags::SHADER_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::GENERAL)
        .image(wind_buffer.shadow_volume_image)
        .subresource_range(shadow_volume_subresource());
    device.cmd_pipeline_barrier(
        cmd,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::PipelineStageFlags::COMPUTE_SHADER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier],
    );
}

unsafe fn insert_post_bake_barrier(
    device: &Device,
    wind_buffer: &WindBuffer,
    cmd: vk::CommandBuffer,
) {
    let barrier = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::SHADER_WRITE)
        .dst_access_mask(vk::AccessFlags::SHADER_READ)
        .old_layout(vk::ImageLayout::GENERAL)
        .new_layout(vk::ImageLayout::GENERAL)
        .image(wind_buffer.shadow_volume_image)
        .subresource_range(shadow_volume_subresource());
    device.cmd_pipeline_barrier(
        cmd,
        vk::PipelineStageFlags::COMPUTE_SHADER,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier],
    );
}

/// Bakes the wall + envelope shadow depths of every drawn instance into its slot of the volume.
pub unsafe fn record_wind_shadow_bake_pass(
    ctx: &FrameRenderContext,
    wind_buffer: &WindBuffer,
    pipeline: &RRPipeline,
    descriptor: &RRWindShadowBakeDescriptorSet,
    draws: &[WindInstanceDraw],
    image_index: usize,
    cmd: vk::CommandBuffer,
) -> Result<()> {
    let device = &ctx.device.device;
    insert_pre_bake_barrier(device, wind_buffer, cmd);

    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, pipeline.pipeline);
    let extent = wind_shadow_volume_extent();
    let group_count_x =
        (extent.width / WIND_SHADOW_VOLUME_SLOTS).div_ceil(SHADOW_BAKE_WORKGROUP_SIZE);
    let group_count_y = extent.height.div_ceil(SHADOW_BAKE_WORKGROUP_SIZE);
    let frame_set = ctx.graphics.frame_set.sets[image_index];
    for draw in draws {
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::COMPUTE,
            pipeline.pipeline_layout,
            0,
            &[frame_set, descriptor.descriptor_set],
            &[draw.ubo_dynamic_offset],
        );
        device.cmd_dispatch(cmd, group_count_x, group_count_y, extent.depth);
    }

    insert_post_bake_barrier(device, wind_buffer, cmd);
    Ok(())
}

pub unsafe fn record_wind_half_resolve_pass(
    ctx: &FrameRenderContext,
    wind_buffer: &WindBuffer,
    pipeline: &RRPipeline,
    descriptor: &RRWindDescriptorSet,
    draws: &[WindInstanceDraw],
    push_constants: WindPushConstants,
    image_index: usize,
    cmd: vk::CommandBuffer,
) -> Result<()> {
    let device = &ctx.device.device;
    let half_extent = wind_buffer.half_extent();

    let clear_values = [vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 0.0],
        },
    }];
    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(wind_buffer.half_render_pass)
        .framebuffer(wind_buffer.half_framebuffer)
        .render_area(vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: half_extent,
        })
        .clear_values(&clear_values);
    device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);

    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(half_extent.width as f32)
        .height(half_extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);
    device.cmd_set_viewport(cmd, 0, &[viewport]);
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
            &[draw.ubo_dynamic_offset],
        );
        device.cmd_draw(cmd, 3, 1, 0, 0);
    }

    device.cmd_end_render_pass(cmd);
    Ok(())
}

pub unsafe fn record_wind_upsample_pass(
    ctx: &FrameRenderContext,
    wind_buffer: &WindBuffer,
    pipeline: &RRPipeline,
    descriptor: &RRWindUpsampleDescriptorSet,
    scissor: vk::Rect2D,
    cmd: vk::CommandBuffer,
) -> Result<()> {
    let device = &ctx.device.device;

    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(wind_buffer.render_pass)
        .framebuffer(wind_buffer.framebuffer)
        .render_area(scissor);
    device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);

    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(wind_buffer.width as f32)
        .height(wind_buffer.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);
    device.cmd_set_viewport(cmd, 0, &[viewport]);
    device.cmd_set_scissor(cmd, 0, &[scissor]);

    device.cmd_bind_descriptor_sets(
        cmd,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline_layout,
        0,
        &[descriptor.descriptor_set],
        &[],
    );
    device.cmd_draw(cmd, 3, 1, 0, 0);

    device.cmd_end_render_pass(cmd);
    Ok(())
}

pub unsafe fn record_wind_shading_pass(
    ctx: &FrameRenderContext,
    wind_buffer: &WindBuffer,
    pipeline: &RRPipeline,
    descriptor: &RRWindDescriptorSet,
    ubo_dynamic_offset: u32,
    scissor: vk::Rect2D,
    push_constants: WindPushConstants,
    image_index: usize,
    cmd: vk::CommandBuffer,
) -> Result<()> {
    let device = &ctx.device.device;

    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(wind_buffer.render_pass)
        .framebuffer(wind_buffer.framebuffer)
        .render_area(scissor);
    device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);

    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(wind_buffer.width as f32)
        .height(wind_buffer.height as f32)
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
        &[frame_set, descriptor.descriptor_set],
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
