use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::shader_bindings::lightning_resolve;
use crate::vulkanr::descriptor::{
    ReflectedDescriptorSets, ReflectedLayoutSpec, ReflectedSetLayout, LIGHTNING_RESOLVE,
};
use crate::vulkanr::image::create_scene_depth_sampler;
use crate::vulkanr::resource::GpuResource;
use crate::vulkanr::resource::UniformBuffer;
use thyllore_effect_core::{LightningSegmentsUBO, LightningUBO};

#[derive(Clone, Debug, Default)]
pub struct LightningResolveDescriptorSet {
    sets: ReflectedDescriptorSets,
    pub scene_depth_sampler: vk::Sampler,
}

impl LightningResolveDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&LIGHTNING_RESOLVE)
            .with_override(
                lightning_resolve::LIGHTNING,
                vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
            )
            .with_override(
                lightning_resolve::SEGMENTS,
                vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
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

    pub fn descriptor_set(&self) -> vk::DescriptorSet {
        self.sets.set(0)
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        lightning_ubo: &UniformBuffer<LightningUBO>,
        segments_ubo: &UniformBuffer<LightningSegmentsUBO>,
        scene_depth_view: vk::ImageView,
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
        scene_depth_view: vk::ImageView,
    ) -> Result<()> {
        self.sets
            .writer(0)
            .image(
                lightning_resolve::SCENE_DEPTH_SAMPLER,
                scene_depth_view,
                self.scene_depth_sampler,
                vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        self.sets.destroy(device);
        device.destroy_sampler(self.scene_depth_sampler, None);
    }
}

impl GpuResource for LightningResolveDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device);
    }
}
