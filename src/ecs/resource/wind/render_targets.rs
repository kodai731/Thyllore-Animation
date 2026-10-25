use thyllore_effect_core::WindUBO;
use thyllore_vulkan_core::descriptor::OverlayUpsampleDescriptorSet;
use thyllore_vulkan_core::effect::wind::targets::WindRenderTargets;
use thyllore_vulkan_core::pipeline::RRPipeline;
use thyllore_vulkan_core::resource::{GpuResource, UniformBuffer};

use crate::ecs::resource::BoundGenerations;
use crate::gpu_resource;
use thyllore_vulkan_core::effect::wind::descriptors::{
    WindResolveDescriptorSet, WindShadowBakeDescriptorSet,
};

/// Pipelines, descriptor sets and the per-instance UBO of the wind passes.
#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct WindGpuState {
    pub ubo: Option<UniformBuffer<WindUBO>>,
    pub resolve_pipeline: Option<RRPipeline>,
    pub resolve_descriptor: Option<WindResolveDescriptorSet>,
    pub shadow_bake_pipeline: Option<RRPipeline>,
    pub shadow_bake_descriptor: Option<WindShadowBakeDescriptorSet>,
    pub upsample_pipeline: Option<RRPipeline>,
    pub upsample_descriptor: Option<OverlayUpsampleDescriptorSet>,
    #[gpu_resource(skip)]
    pub upsample_bound: BoundGenerations,
}

gpu_resource!(WindGpuState);
gpu_resource!(WindRenderTargets);
