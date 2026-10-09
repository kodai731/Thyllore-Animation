use thyllore_vulkan_core::pipeline::RRRayTracingPipeline;
use thyllore_vulkan_core::resource::GpuResource;

use crate::ecs::systems::RREffectTraceDescriptorSet;
use crate::gpu_resource;

#[derive(Debug, Default, GpuResource)]
pub struct EffectTraceGpuState {
    pub pipeline: Option<RRRayTracingPipeline>,
    pub descriptor: Option<RREffectTraceDescriptorSet>,
}

gpu_resource!(EffectTraceGpuState);
