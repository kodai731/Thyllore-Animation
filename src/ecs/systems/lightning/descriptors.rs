use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::shader_bindings::lightning_resolve;
use crate::vulkanr::descriptor::{
    DescriptorKind, ReflectedLayoutSpec, ReflectedSetLayout, LIGHTNING_RESOLVE,
};
use crate::vulkanr::image::create_scene_depth_sampler;
use thyllore_effect_core::{LightningSegmentsUBO, LightningUBO};
use thyllore_vulkan_core::resource::{GpuResource, UniformBuffer};

#[derive(Clone, Debug, Default)]
pub struct LightningResolveDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub(crate) descriptor_set: thyllore_vulkan_core::types::DescriptorSet,
    pub scene_depth_sampler: thyllore_vulkan_core::types::Sampler,
}

impl LightningResolveDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&LIGHTNING_RESOLVE)
            .with_override(
                lightning_resolve::LIGHTNING,
                DescriptorKind::UniformBufferDynamic,
            )
            .with_override(
                lightning_resolve::SEGMENTS,
                DescriptorKind::UniformBufferDynamic,
            )
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let descriptor_set = layout.allocate_descriptor_set(rrdevice)?;
        let scene_depth_sampler = create_scene_depth_sampler(rrdevice)?;

        Ok(Self {
            layout,
            descriptor_set,
            scene_depth_sampler,
        })
    }

    pub fn descriptor_set(&self) -> thyllore_vulkan_core::types::DescriptorSet {
        self.descriptor_set
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        lightning_ubo: &UniformBuffer<LightningUBO>,
        segments_ubo: &UniformBuffer<LightningSegmentsUBO>,
        scene_depth_view: thyllore_vulkan_core::types::ImageView,
    ) -> Result<()> {
        self.layout
            .descriptor_writer(self.descriptor_set)
            .uniform_dynamic(lightning_resolve::LIGHTNING, lightning_ubo)?
            .uniform_dynamic(lightning_resolve::SEGMENTS, segments_ubo)?
            .apply(rrdevice);
        self.update_scene_depth(rrdevice, scene_depth_view)
    }

    pub unsafe fn update_scene_depth(
        &self,
        rrdevice: &RRDevice,
        scene_depth_view: thyllore_vulkan_core::types::ImageView,
    ) -> Result<()> {
        self.layout
            .descriptor_writer(self.descriptor_set)
            .sampled_depth(
                lightning_resolve::SCENE_DEPTH_SAMPLER,
                scene_depth_view,
                self.scene_depth_sampler,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.layout.destroy(rrdevice);
        self.scene_depth_sampler.destroy_gpu(rrdevice);
    }
}

impl GpuResource for LightningResolveDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.layout.is_destroyed() && self.scene_depth_sampler.is_destroyed()
    }
}
