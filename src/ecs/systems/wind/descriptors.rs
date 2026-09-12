use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::shader_bindings::{wind_resolve, wind_shadow_bake, wind_upsample};
use crate::vulkanr::descriptor::{
    ReflectedLayoutSpec, ReflectedSetLayout, WIND_RESOLVE, WIND_SHADOW_BAKE, WIND_UPSAMPLE,
};
use crate::vulkanr::image::{create_nearest_sampler, create_scene_depth_sampler};
use crate::vulkanr::resource::{UniformBuffer, VolumeImage};
use thyllore_effect_core::WindUBO;

#[derive(Clone, Debug, Default)]
pub struct WindResolveDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub descriptor_set: vk::DescriptorSet,
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
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let descriptor_set = layout.allocate_set(rrdevice)?;
        let scene_depth_sampler = create_scene_depth_sampler(rrdevice)?;

        Ok(Self {
            layout,
            descriptor_set,
            scene_depth_sampler,
        })
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        wind_ubo: &UniformBuffer<WindUBO>,
        shadow_volume: &VolumeImage,
        scene_depth_view: vk::ImageView,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
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
        self.layout
            .writer(self.descriptor_set)
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
        self.layout
            .writer(self.descriptor_set)
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
        self.layout.destroy(device);
        device.destroy_sampler(self.scene_depth_sampler, None);
    }
}

#[derive(Clone, Debug, Default)]
pub struct WindShadowBakeDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub descriptor_set: vk::DescriptorSet,
}

impl WindShadowBakeDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WIND_SHADOW_BAKE).with_override(
            wind_shadow_bake::WIND,
            vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
        )
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let descriptor_set = layout.allocate_set(rrdevice)?;

        Ok(Self {
            layout,
            descriptor_set,
        })
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        wind_ubo: &UniformBuffer<WindUBO>,
        shadow_volume: &VolumeImage,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
            .uniform_dynamic(wind_shadow_bake::WIND, wind_ubo)?
            .apply(rrdevice);
        self.update_shadow_volume(rrdevice, shadow_volume)
    }

    pub unsafe fn update_shadow_volume(
        &self,
        rrdevice: &RRDevice,
        shadow_volume: &VolumeImage,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
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
        self.layout.destroy(device);
    }
}

#[derive(Clone, Debug, Default)]
pub struct WindUpsampleDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub descriptor_set: vk::DescriptorSet,
    pub wind_color_sampler: vk::Sampler,
    pub scene_depth_sampler: vk::Sampler,
}

impl WindUpsampleDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WIND_UPSAMPLE)
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let descriptor_set = layout.allocate_set(rrdevice)?;
        let wind_color_sampler = create_nearest_sampler(rrdevice)?;
        let scene_depth_sampler = create_scene_depth_sampler(rrdevice)?;

        Ok(Self {
            layout,
            descriptor_set,
            wind_color_sampler,
            scene_depth_sampler,
        })
    }

    pub unsafe fn update_image_views(
        &self,
        rrdevice: &RRDevice,
        wind_color_view: vk::ImageView,
        scene_depth_view: vk::ImageView,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
            .image(
                wind_upsample::WIND_COLOR_SAMPLER,
                wind_color_view,
                self.wind_color_sampler,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            )?
            .image(
                wind_upsample::SCENE_DEPTH_SAMPLER,
                scene_depth_view,
                self.scene_depth_sampler,
                vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        self.layout.destroy(device);
        device.destroy_sampler(self.wind_color_sampler, None);
        device.destroy_sampler(self.scene_depth_sampler, None);
    }
}
