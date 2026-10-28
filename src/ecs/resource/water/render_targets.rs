pub use crate::ecs::systems::water::gpu_targets::WaterRenderTargets;
use thyllore_effect_core::WaterUBO;
use thyllore_vulkan_core::pipeline::RRPipeline;
use thyllore_vulkan_core::resource::{GpuResource, UniformBuffer};
use thyllore_vulkan_core::types::AccelerationStructure;

use crate::ecs::systems::water::{RRWaterCausticDescriptorSet, RRWaterDescriptorSet};
use crate::gpu_resource;

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct WaterGpuState {
    pub shading_pipeline: Option<RRPipeline>,
    pub descriptor: Option<RRWaterDescriptorSet>,
    pub ubo: Option<UniformBuffer<WaterUBO>>,
    pub caustic_splat_pipeline: Option<RRPipeline>,
    pub caustic_apply_pipeline: Option<RRPipeline>,
    pub caustic_descriptor: Option<RRWaterCausticDescriptorSet>,
    #[gpu_resource(skip)]
    pub caustic_bound_tlas: AccelerationStructure,
}

gpu_resource!(WaterGpuState);
