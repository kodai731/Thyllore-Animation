use thyllore_vulkan_core::descriptor::ReflectedDescriptorSets;
use thyllore_vulkan_core::pipeline::RRRayTracingPipeline;
use thyllore_vulkan_core::resource::GpuResource;

use crate::gpu_resource;

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct EffectTraceGpuState {
    pub pipeline: Option<RRRayTracingPipeline>,
    pub descriptor: Option<ReflectedDescriptorSets>,
}

gpu_resource!(EffectTraceGpuState);
