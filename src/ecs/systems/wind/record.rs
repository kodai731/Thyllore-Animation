use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::WindRenderTargets;
use crate::ecs::systems::wind::descriptors::WindShadowBakeDescriptorSet;
use crate::ecs::systems::wind::render_targets::WIND_SHADOW_VOLUME;
use crate::vulkanr::pipeline::RRPipeline;
use thyllore_vulkan_core::{
    record_shadow_volume_bake, FrameRenderContext, OverlayInstanceDraw, ShadowVolumeBake,
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
) {
    let descriptor_sets = [
        ctx.graphics.frame_set.sets[image_index],
        descriptor.descriptor_set,
    ];
    let bake = ShadowVolumeBake {
        volume: &targets.shadow_volume,
        spec: &WIND_SHADOW_VOLUME,
        pipeline,
        descriptor_sets: &descriptor_sets,
        workgroup_size: SHADOW_BAKE_WORKGROUP_SIZE,
        reader_stage: vk::PipelineStageFlags::FRAGMENT_SHADER,
    };
    record_shadow_volume_bake(
        &ctx.device.device,
        cmd,
        &bake,
        draws.iter().map(|draw| draw.dynamic_offsets.as_slice()),
    );
}
