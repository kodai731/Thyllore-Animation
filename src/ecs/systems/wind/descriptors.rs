use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::shader_bindings::{wind_resolve, wind_shadow_bake};
use crate::vulkanr::descriptor::{
    DescriptorKind, ReflectedLayoutSpec, ReflectedSetLayout, WIND_RESOLVE, WIND_SHADOW_BAKE,
};
use crate::vulkanr::image::create_scene_depth_sampler;
use thyllore_effect_core::WindUBO;
use thyllore_vulkan_core::resource::{GpuResource, UniformBuffer, VolumeImage};

#[derive(Clone, Debug, Default)]
pub struct WindResolveDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub(crate) descriptor_set: thyllore_vulkan_core::types::DescriptorSet,
    pub scene_depth_sampler: thyllore_vulkan_core::types::Sampler,
}

impl WindResolveDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WIND_RESOLVE)
            .with_override(wind_resolve::WIND, DescriptorKind::UniformBufferDynamic)
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
        wind_ubo: &UniformBuffer<WindUBO>,
        shadow_volume: &VolumeImage,
        scene_depth_view: thyllore_vulkan_core::types::ImageView,
    ) -> Result<()> {
        self.layout
            .descriptor_writer(self.descriptor_set)
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
            .descriptor_writer(self.descriptor_set)
            .sampled_storage_image(
                wind_resolve::SHADOW_VOLUME_SAMPLER,
                shadow_volume.view(),
                shadow_volume.sampler(),
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn update_scene_depth(
        &self,
        rrdevice: &RRDevice,
        scene_depth_view: thyllore_vulkan_core::types::ImageView,
    ) -> Result<()> {
        self.layout
            .descriptor_writer(self.descriptor_set)
            .sampled_depth(
                wind_resolve::SCENE_DEPTH_SAMPLER,
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

#[derive(Clone, Debug, Default)]
pub struct WindShadowBakeDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub descriptor_set: thyllore_vulkan_core::types::DescriptorSet,
}

impl WindShadowBakeDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WIND_SHADOW_BAKE)
            .with_override(wind_shadow_bake::WIND, DescriptorKind::UniformBufferDynamic)
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let descriptor_set = layout.allocate_descriptor_set(rrdevice)?;

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
            .descriptor_writer(self.descriptor_set)
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
            .descriptor_writer(self.descriptor_set)
            .storage_image(wind_shadow_bake::SHADOW_VOLUME_IMAGE, shadow_volume.view())?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.layout.destroy(rrdevice);
    }
}

impl GpuResource for WindResolveDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.layout.is_destroyed() && self.scene_depth_sampler.is_destroyed()
    }
}

impl GpuResource for WindShadowBakeDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.layout.is_destroyed()
    }
}
