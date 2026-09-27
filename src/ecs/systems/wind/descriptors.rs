use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::shader_bindings::{wind_resolve, wind_shadow_bake};
use crate::vulkanr::descriptor::{
    ReflectedDescriptorSets, ReflectedLayoutSpec, WIND_RESOLVE, WIND_SHADOW_BAKE,
};
use crate::vulkanr::image::create_scene_depth_sampler;
use crate::vulkanr::resource::GpuResource;
use crate::vulkanr::resource::{UniformBuffer, VolumeImage};
use thyllore_effect_core::WindUBO;

#[derive(Clone, Debug, Default)]
pub struct WindResolveDescriptorSet {
    sets: ReflectedDescriptorSets,
    pub scene_depth_sampler: vk::Sampler,
}

impl WindResolveDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WIND_RESOLVE).with_override(
            wind_resolve::WIND,
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

    pub fn layout(&self) -> &crate::vulkanr::descriptor::ReflectedSetLayout {
        self.sets.layout()
    }

    pub fn descriptor_set(&self) -> vk::DescriptorSet {
        self.sets.set(0)
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        wind_ubo: &UniformBuffer<WindUBO>,
        shadow_volume: &VolumeImage,
        scene_depth_view: vk::ImageView,
    ) -> Result<()> {
        self.sets
            .writer(0)
            .uniform_dynamic(wind_resolve::WIND, wind_ubo)?
            .apply(rrdevice);
        self.update_shadow_volume(rrdevice, shadow_volume)?;
        self.update_scene_depth(rrdevice, scene_depth_view)
    }

    pub unsafe fn update_shadow_volume(
        &self,
        rrdevice: &RRDevice,
        shadow_volume: &VolumeImage,
    ) -> Result<()> {
        self.sets
            .writer(0)
            .image(
                wind_resolve::SHADOW_VOLUME_SAMPLER,
                shadow_volume.view,
                shadow_volume.sampler,
                vk::ImageLayout::GENERAL,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn update_scene_depth(
        &self,
        rrdevice: &RRDevice,
        scene_depth_view: vk::ImageView,
    ) -> Result<()> {
        self.sets
            .writer(0)
            .image(
                wind_resolve::SCENE_DEPTH_SAMPLER,
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

#[derive(Clone, Debug, Default)]
pub struct WindShadowBakeDescriptorSet {
    sets: ReflectedDescriptorSets,
}

impl WindShadowBakeDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WIND_SHADOW_BAKE).with_override(
            wind_shadow_bake::WIND,
            vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
        )
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let sets = ReflectedDescriptorSets::create(rrdevice, &Self::layout_spec(), 1)?;
        Ok(Self { sets })
    }

    pub fn layout(&self) -> &crate::vulkanr::descriptor::ReflectedSetLayout {
        self.sets.layout()
    }

    pub fn descriptor_set(&self) -> vk::DescriptorSet {
        self.sets.set(0)
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        wind_ubo: &UniformBuffer<WindUBO>,
        shadow_volume: &VolumeImage,
    ) -> Result<()> {
        self.sets
            .writer(0)
            .uniform_dynamic(wind_shadow_bake::WIND, wind_ubo)?
            .apply(rrdevice);
        self.update_shadow_volume(rrdevice, shadow_volume)
    }

    pub unsafe fn update_shadow_volume(
        &self,
        rrdevice: &RRDevice,
        shadow_volume: &VolumeImage,
    ) -> Result<()> {
        self.sets
            .writer(0)
            .image(
                wind_shadow_bake::SHADOW_VOLUME_IMAGE,
                shadow_volume.view,
                vk::Sampler::null(),
                vk::ImageLayout::GENERAL,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        self.sets.destroy(device);
    }
}

impl GpuResource for WindResolveDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device);
    }
}

impl GpuResource for WindShadowBakeDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device);
    }
}
