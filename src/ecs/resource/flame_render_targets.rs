use crate::ecs::systems::flame::RRFlameDescriptorSet;
use thyllore_effect_core::FlameUBO;
use thyllore_vulkan_core::pipeline::RRPipeline;
use thyllore_vulkan_core::resource::{FlameBuffer, GpuResource, RRImage, UniformBuffer};

use crate::gpu_resource;

#[derive(Debug, Default, GpuResource)]
pub struct FlameRenderTargets {
    pub buffer: FlameBuffer,
}

#[derive(Debug, Default, GpuResource)]
pub struct FlameGpuState {
    pub shading_pipeline: Option<RRPipeline>,
    pub descriptor: Option<RRFlameDescriptorSet>,
    pub ubo: Option<UniformBuffer<FlameUBO>>,
    pub sdf: RRImage,
}

gpu_resource!(FlameRenderTargets);
gpu_resource!(FlameGpuState);
