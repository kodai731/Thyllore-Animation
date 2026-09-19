use vulkanalia::prelude::v1_0::*;

use crate::ecs::systems::flame::RRFlameDescriptorSet;
use crate::gpu_resource;
use thyllore_effect_core::FlameUBO;
use thyllore_vulkan_core::pipeline::RRPipeline;
use thyllore_vulkan_core::resource::{GpuResource, RRImage, UniformBuffer};

#[derive(Clone, Debug, Default)]
pub struct FlameBuffer {
    pub history_images: [vk::Image; 2],
    pub history_image_views: [vk::ImageView; 2],
    pub sampler: vk::Sampler,
    pub shading_render_pass: vk::RenderPass,
    pub shading_framebuffers: [vk::Framebuffer; 2],
    pub width: u32,
    pub height: u32,
}

impl FlameBuffer {
    pub fn extent(&self) -> vk::Extent2D {
        vk::Extent2D {
            width: self.width,
            height: self.height,
        }
    }
}

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
