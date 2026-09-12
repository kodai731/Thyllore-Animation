use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::WindRenderTargets;
use crate::ecs::systems::wind::descriptors::{
    WindResolveDescriptorSet, WindShadowBakeDescriptorSet, WindUpsampleDescriptorSet,
};
use crate::ecs::systems::wind::render_targets::wind_shadow_volume_extent;
use crate::vulkanr::pipeline::RRPipeline;
use thyllore_effect_core::WIND_SHADOW_VOLUME_SLOTS;
use thyllore_vulkan_core::FrameRenderContext;

/// Must match local_size in shadowBake.comp.
const SHADOW_BAKE_WORKGROUP_SIZE: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct WindPushConstants {
    pub mode: i32,
    pub step_count: i32,
    pub debug_view: i32,
}

impl WindPushConstants {
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

pub struct WindInstanceDraw {
    pub ubo_dynamic_offset: u32,
    pub scissor: vk::Rect2D,
}

/// The volume is rewritten in full every frame, so the previous contents are discarded.
unsafe fn insert_pre_bake_barrier(
    device: &Device,
    targets: &WindRenderTargets,
    cmd: vk::CommandBuffer,
) {
    let barrier = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::SHADER_READ)
        .dst_access_mask(vk::AccessFlags::SHADER_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::GENERAL)
        .image(targets.shadow_volume.image)
        .subresource_range(targets.shadow_volume.subresource_range());
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
    targets: &WindRenderTargets,
    cmd: vk::CommandBuffer,
) {
    let barrier = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::SHADER_WRITE)
        .dst_access_mask(vk::AccessFlags::SHADER_READ)
        .old_layout(vk::ImageLayout::GENERAL)
        .new_layout(vk::ImageLayout::GENERAL)
        .image(targets.shadow_volume.image)
        .subresource_range(targets.shadow_volume.subresource_range());
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
    targets: &WindRenderTargets,
    pipeline: &RRPipeline,
    descriptor: &WindShadowBakeDescriptorSet,
    draws: &[WindInstanceDraw],
    image_index: usize,
    cmd: vk::CommandBuffer,
) -> Result<()> {
    let device = &ctx.device.device;
    insert_pre_bake_barrier(device, targets, cmd);

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

    insert_post_bake_barrier(device, targets, cmd);
    Ok(())
}

unsafe fn set_full_viewport(device: &Device, cmd: vk::CommandBuffer, extent: vk::Extent2D) {
    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(extent.width as f32)
        .height(extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);
    device.cmd_set_viewport(cmd, 0, &[viewport]);
}

pub unsafe fn record_wind_half_resolve_pass(
    ctx: &FrameRenderContext,
    targets: &WindRenderTargets,
    pipeline: &RRPipeline,
    descriptor: &WindResolveDescriptorSet,
    draws: &[WindInstanceDraw],
    push_constants: WindPushConstants,
    image_index: usize,
    cmd: vk::CommandBuffer,
) -> Result<()> {
    let device = &ctx.device.device;
    let half_extent = targets.half_extent();

    let clear_values = [vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 0.0],
        },
    }];
    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(targets.half_render_pass)
        .framebuffer(targets.half_framebuffer)
        .render_area(vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: half_extent,
        })
        .clear_values(&clear_values);
    device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);

    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
    set_full_viewport(device, cmd, half_extent);
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
    targets: &WindRenderTargets,
    pipeline: &RRPipeline,
    descriptor: &WindUpsampleDescriptorSet,
    scissor: vk::Rect2D,
    cmd: vk::CommandBuffer,
) -> Result<()> {
    let device = &ctx.device.device;

    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(targets.render_pass)
        .framebuffer(targets.framebuffer)
        .render_area(scissor);
    device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);

    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
    set_full_viewport(device, cmd, targets.extent());
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
    targets: &WindRenderTargets,
    pipeline: &RRPipeline,
    descriptor: &WindResolveDescriptorSet,
    draw: &WindInstanceDraw,
    push_constants: WindPushConstants,
    image_index: usize,
    cmd: vk::CommandBuffer,
) -> Result<()> {
    let device = &ctx.device.device;

    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(targets.render_pass)
        .framebuffer(targets.framebuffer)
        .render_area(draw.scissor);
    device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);

    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
    set_full_viewport(device, cmd, targets.extent());
    device.cmd_set_scissor(cmd, 0, &[draw.scissor]);

    let frame_set = ctx.graphics.frame_set.sets[image_index];
    device.cmd_bind_descriptor_sets(
        cmd,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline_layout,
        0,
        &[frame_set, descriptor.descriptor_set],
        &[draw.ubo_dynamic_offset],
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
