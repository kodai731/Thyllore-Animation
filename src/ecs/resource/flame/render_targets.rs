use crate::ecs::systems::flame::RRFlameDescriptorSet;
use crate::gpu_resource;
use thyllore_effect_core::FlameUBO;
use thyllore_vulkan_core::pipeline::RRPipeline;
use thyllore_vulkan_core::resource::{GpuResource, HistoryTargets, RRImage, UniformBuffer};

#[derive(Debug, Default, GpuResource)]
pub struct FlameRenderTargets {
    pub history: HistoryTargets,
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
