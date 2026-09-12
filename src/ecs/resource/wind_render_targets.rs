use thyllore_effect_core::WindUBO;
use thyllore_vulkan_core::pipeline::RRPipeline;
use thyllore_vulkan_core::resource::{UniformBuffer, VolumeImage};
use thyllore_vulkan_core::vulkan::vk;

use crate::ecs::systems::wind::{
    WindResolveDescriptorSet, WindShadowBakeDescriptorSet, WindUpsampleDescriptorSet,
};

/// Render pass and framebuffer that blend the wind resolve pass onto the HDR color image,
/// the half resolution intermediate target used by the upsampled resolve path, and the
/// shadow volume baked once per frame for every instance slot.
#[derive(Debug, Default)]
pub struct WindRenderTargets {
    pub render_pass: vk::RenderPass,
    pub framebuffer: vk::Framebuffer,
    pub half_render_pass: vk::RenderPass,
    pub half_framebuffer: vk::Framebuffer,
    pub half_color_image: vk::Image,
    pub half_color_image_memory: vk::DeviceMemory,
    pub half_color_image_view: vk::ImageView,
    pub shadow_volume: VolumeImage,
    pub width: u32,
    pub height: u32,
}

impl WindRenderTargets {
    pub fn extent(&self) -> vk::Extent2D {
        vk::Extent2D {
            width: self.width,
            height: self.height,
        }
    }

    pub fn half_extent(&self) -> vk::Extent2D {
        half_extent(self.width, self.height)
    }
}

pub fn half_extent(width: u32, height: u32) -> vk::Extent2D {
    vk::Extent2D {
        width: (width / 2).max(1),
        height: (height / 2).max(1),
    }
}

/// Pipelines, descriptor sets and the per-instance UBO of the wind passes.
#[derive(Debug, Default)]
pub struct WindGpuState {
    pub ubo: Option<UniformBuffer<WindUBO>>,
    pub resolve_pipeline: Option<RRPipeline>,
    pub resolve_descriptor: Option<WindResolveDescriptorSet>,
    pub shadow_bake_pipeline: Option<RRPipeline>,
    pub shadow_bake_descriptor: Option<WindShadowBakeDescriptorSet>,
    pub upsample_pipeline: Option<RRPipeline>,
    pub upsample_descriptor: Option<WindUpsampleDescriptorSet>,
}
