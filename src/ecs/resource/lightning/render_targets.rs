use thyllore_effect_core::{LightningSegmentsUBO, LightningUBO};
use thyllore_vulkan_core::pipeline::RRPipeline;
use thyllore_vulkan_core::resource::{GpuResource, UniformBuffer};
use thyllore_vulkan_core::vulkan::vk;

use crate::ecs::systems::lightning::LightningResolveDescriptorSet;
use crate::gpu_resource;

/// Render pass and framebuffer that blend the lightning resolve pass onto the HDR color image.
#[derive(Debug, Default)]
pub struct LightningRenderTargets {
    pub render_pass: vk::RenderPass,
    pub framebuffer: vk::Framebuffer,
    pub width: u32,
    pub height: u32,
}

impl LightningRenderTargets {
    pub fn extent(&self) -> vk::Extent2D {
        vk::Extent2D {
            width: self.width,
            height: self.height,
        }
    }
}

/// Pipeline, descriptor set and the per-instance UBOs of the lightning resolve pass.
#[derive(Debug, Default, GpuResource)]
pub struct LightningGpuState {
    pub ubo: Option<UniformBuffer<LightningUBO>>,
    pub segments_ubo: Option<UniformBuffer<LightningSegmentsUBO>>,
    pub resolve_pipeline: Option<RRPipeline>,
    pub resolve_descriptor: Option<LightningResolveDescriptorSet>,
}

gpu_resource!(LightningRenderTargets);
gpu_resource!(LightningGpuState);
