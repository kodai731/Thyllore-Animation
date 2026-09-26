use thyllore_effect_core::WindUBO;
use thyllore_vulkan_core::pipeline::RRPipeline;
use thyllore_vulkan_core::resource::hdr_buffer::HDR_FORMAT;
use thyllore_vulkan_core::resource::render_target_transient::TransientDesc;
use thyllore_vulkan_core::resource::{GpuResource, UniformBuffer, VolumeImage};
use thyllore_vulkan_core::vulkan::vk;

use crate::ecs::resource::BoundGenerations;
use crate::ecs::systems::wind::{
    WindResolveDescriptorSet, WindShadowBakeDescriptorSet, WindUpsampleDescriptorSet,
};
use crate::gpu_resource;

/// Render pass and framebuffer that blend the wind resolve pass onto the HDR color image,
/// the half resolution render pass used by the upsampled resolve path, and the
/// shadow volume baked once per frame for every instance slot.
#[derive(Debug, Default)]
pub struct WindRenderTargets {
    pub render_pass: vk::RenderPass,
    pub framebuffer: vk::Framebuffer,
    pub half_render_pass: vk::RenderPass,
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

    pub fn half_color_desc(&self) -> TransientDesc {
        let extent = self.half_extent();
        TransientDesc {
            width: extent.width,
            height: extent.height,
            format: HDR_FORMAT,
            usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
        }
    }
}

pub fn half_extent(width: u32, height: u32) -> vk::Extent2D {
    vk::Extent2D {
        width: (width / 2).max(1),
        height: (height / 2).max(1),
    }
}

/// Pipelines, descriptor sets and the per-instance UBO of the wind passes.
#[derive(Debug, Default, GpuResource)]
pub struct WindGpuState {
    pub ubo: Option<UniformBuffer<WindUBO>>,
    pub resolve_pipeline: Option<RRPipeline>,
    pub resolve_descriptor: Option<WindResolveDescriptorSet>,
    pub shadow_bake_pipeline: Option<RRPipeline>,
    pub shadow_bake_descriptor: Option<WindShadowBakeDescriptorSet>,
    pub upsample_pipeline: Option<RRPipeline>,
    pub upsample_descriptor: Option<WindUpsampleDescriptorSet>,
    #[gpu_resource(skip)]
    pub upsample_bound: BoundGenerations,
}

gpu_resource!(WindRenderTargets);
gpu_resource!(WindGpuState);
