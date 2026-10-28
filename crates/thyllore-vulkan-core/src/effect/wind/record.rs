use anyhow::Result;

use crate::effect::wind::descriptors::WindShadowBakeDescriptorSet;
use crate::effect::wind::targets::{wind_shadow_volume_extent, WindRenderTargets};
use crate::pipeline::RRPipeline;
use crate::renderer::{
    bind_compute, dispatch, storage_image_read_barrier_for_fragment,
    storage_image_write_barrier_for_fragment,
};
use crate::types::{CommandBuffer, DescriptorSet};
use crate::{FrameRenderContext, OverlayInstanceDraw};
use thyllore_effect_core::WIND_SHADOW_VOLUME_SLOTS;

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
    cmd: CommandBuffer,
) -> Result<()> {
    let volume = &targets.shadow_volume;
    storage_image_write_barrier_for_fragment(&ctx.device, cmd, volume);

    let extent = wind_shadow_volume_extent();
    let group_count_x =
        (extent.width / WIND_SHADOW_VOLUME_SLOTS).div_ceil(SHADOW_BAKE_WORKGROUP_SIZE);
    let group_count_y = extent.height.div_ceil(SHADOW_BAKE_WORKGROUP_SIZE);
    let frame_set = ctx.graphics.frame_set.sets[image_index];
    let bake_set = DescriptorSet(descriptor.descriptor_set);
    for draw in draws {
        bind_compute(
            &ctx.device,
            cmd,
            pipeline,
            &[frame_set, bake_set],
            &draw.dynamic_offsets,
        );
        dispatch(&ctx.device, cmd, group_count_x, group_count_y, extent.depth);
    }

    storage_image_read_barrier_for_fragment(&ctx.device, cmd, volume);
    Ok(())
}
