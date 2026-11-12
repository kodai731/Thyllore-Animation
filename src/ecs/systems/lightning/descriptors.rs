use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::shader_bindings::lightning_resolve;
use crate::vulkanr::descriptor::{
    DescriptorKind, ReflectedDescriptorSets, ReflectedLayoutSpec, ReflectedSetLayout,
    LIGHTNING_RESOLVE,
};
use crate::vulkanr::image::create_scene_depth_sampler;
use thyllore_effect_core::{LightningSegmentsUBO, LightningUBO};
use thyllore_vulkan_core::resource::{GpuResource, UniformBuffer};

#[derive(Clone, Debug, Default)]
pub struct LightningResolveDescriptorSet {
    sets: ReflectedDescriptorSets,
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
        let sets = ReflectedDescriptorSets::create(rrdevice, &Self::layout_spec(), 1)?;
        let scene_depth_sampler = create_scene_depth_sampler(rrdevice)?;

        Ok(Self {
            sets,
            scene_depth_sampler,
        })
    }

    pub fn layout(&self) -> &ReflectedSetLayout {
        self.sets.layout()
    }

    pub fn descriptor_set(&self) -> thyllore_vulkan_core::types::DescriptorSet {
        self.sets.set(0)
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        lightning_ubo: &UniformBuffer<LightningUBO>,
        segments_ubo: &UniformBuffer<LightningSegmentsUBO>,
        scene_depth_view: thyllore_vulkan_core::types::ImageView,
    ) -> Result<()> {
        self.sets
            .writer(0)
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
        self.sets
            .writer(0)
            .sampled_depth(
                lightning_resolve::SCENE_DEPTH_SAMPLER,
                scene_depth_view,
                self.scene_depth_sampler,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.sets.destroy(rrdevice);
        self.scene_depth_sampler.destroy_gpu(rrdevice);
    }
}

impl GpuResource for LightningResolveDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.sets.is_destroyed() && self.scene_depth_sampler.is_destroyed()
    }
}
