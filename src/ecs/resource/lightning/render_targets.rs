use thyllore_effect_core::{LightningSegmentsUBO, LightningUBO};
use thyllore_vulkan_core::pipeline::RRPipeline;
use thyllore_vulkan_core::resource::{GpuResource, UniformBuffer};

use crate::ecs::systems::lightning::descriptors::LightningResolveDescriptorSet;
use crate::ecs::systems::lightning::gpu_targets::LightningRenderTargets;
use crate::gpu_resource;

/// Pipeline, descriptor set and the per-instance UBOs of the lightning resolve pass.
#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct LightningGpuState {
    pub ubo: Option<UniformBuffer<LightningUBO>>,
    pub segments_ubo: Option<UniformBuffer<LightningSegmentsUBO>>,
    pub resolve_pipeline: Option<RRPipeline>,
    pub resolve_descriptor: Option<LightningResolveDescriptorSet>,
}

gpu_resource!(LightningRenderTargets);
gpu_resource!(LightningGpuState);
