use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::WindRenderTargets;
use crate::ecs::systems::wind::descriptors::WindShadowBakeDescriptorSet;
use crate::ecs::systems::wind::render_targets::wind_shadow_volume_extent;
use crate::vulkanr::pipeline::RRPipeline;
use thyllore_effect_core::WIND_SHADOW_VOLUME_SLOTS;
use thyllore_vulkan_core::{
    insert_storage_image_read_barrier, insert_storage_image_write_barrier, FrameRenderContext,
    OverlayInstanceDraw,
};

/// Must match local_size in shadowBake.comp.
const SHADOW_BAKE_WORKGROUP_SIZE: u32 = 8;

/// Bakes the wall + envelope shadow depths of every drawn instance into its slot of the volume.
pub unsafe fn record_wind_shadow_bake_pass(
    ctx: &FrameRenderContext,
    targets: &WindRenderTargets,
    pipeline: &RRPipeline,
    descriptor: &WindShadowBakeDescriptorSet,
    draws: &[OverlayInstanceDraw],
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
            &draw.dynamic_offsets,
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
