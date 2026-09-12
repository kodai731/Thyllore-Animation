use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::WindRenderTargets;
use crate::ecs::systems::wind::descriptors::{
    WindResolveDescriptorSet, WindShadowBakeDescriptorSet, WindUpsampleDescriptorSet,
};
use crate::ecs::systems::wind::render_targets::wind_shadow_volume_extent;
use crate::vulkanr::pipeline::RRPipeline;
use thyllore_effect_core::WIND_SHADOW_VOLUME_SLOTS;
use thyllore_vulkan_core::{
    begin_overlay_render_pass, draw_fullscreen_triangle, insert_storage_image_read_barrier,
    insert_storage_image_write_barrier, set_full_viewport, FrameRenderContext,
    OverlayAttachmentLoad,
};

/// Must match local_size in shadowBake.comp.
const SHADOW_BAKE_WORKGROUP_SIZE: u32 = 8;
const TRANSPARENT_BLACK: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

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

fn full_area(extent: vk::Extent2D) -> vk::Rect2D {
    vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent,
    }
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
    let volume = &targets.shadow_volume;
    insert_storage_image_write_barrier(
        device,
        cmd,
        volume.image,
        volume.subresource_range(),
        vk::PipelineStageFlags::FRAGMENT_SHADER,
    );

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

    insert_storage_image_read_barrier(
        device,
        cmd,
        volume.image,
        volume.subresource_range(),
        vk::PipelineStageFlags::FRAGMENT_SHADER,
    );
    Ok(())
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
    begin_overlay_render_pass(
        device,
        cmd,
        targets.half_render_pass,
        targets.half_framebuffer,
        full_area(half_extent),
        OverlayAttachmentLoad::Clear(TRANSPARENT_BLACK),
    );

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
        draw_fullscreen_triangle(device, cmd);
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
    begin_overlay_render_pass(
        device,
        cmd,
        targets.render_pass,
        targets.framebuffer,
        scissor,
        OverlayAttachmentLoad::Keep,
    );

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
    draw_fullscreen_triangle(device, cmd);

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
    begin_overlay_render_pass(
        device,
        cmd,
        targets.render_pass,
        targets.framebuffer,
        draw.scissor,
        OverlayAttachmentLoad::Keep,
    );

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
    draw_fullscreen_triangle(device, cmd);

    device.cmd_end_render_pass(cmd);
    Ok(())
}
