use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::shader_bindings::{wind_resolve, wind_shadow_bake};
use crate::vulkanr::descriptor::{
    DescriptorKind, ReflectedDescriptorSets, ReflectedLayoutSpec, SetGrid, SetSlot, WIND_RESOLVE,
    WIND_SHADOW_BAKE,
};
use crate::vulkanr::image::create_scene_depth_sampler;
use thyllore_effect_core::WindUBO;
use thyllore_vulkan_core::resource::{build_gpu_resource, GpuResource, UniformBuffer, VolumeImage};
use thyllore_vulkan_core::types::{ImageView, Sampler};

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct WindResolveDescriptorSet {
    pub sets: ReflectedDescriptorSets,
    scene_depth_sampler: Sampler,
}

impl WindResolveDescriptorSet {
    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout_spec = ReflectedLayoutSpec::local(&WIND_RESOLVE)
            .with_override(wind_resolve::WIND, DescriptorKind::UniformBufferDynamic);

        build_gpu_resource(rrdevice, |descriptor: &mut Self| {
            descriptor.sets =
                ReflectedDescriptorSets::create(rrdevice, &layout_spec, SetGrid::SINGLE)?;
            descriptor.scene_depth_sampler = create_scene_depth_sampler(rrdevice)?;
            Ok(())
        })
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        wind_ubo: &UniformBuffer<WindUBO>,
        shadow_volume: &VolumeImage,
        scene_depth_view: ImageView,
    ) -> Result<()> {
        self.sets
            .writer(SetSlot::SINGLE)?
            .uniform_dynamic(wind_resolve::WIND, wind_ubo)?
            .sampled_storage_image(
                wind_resolve::SHADOW_VOLUME_SAMPLER,
                shadow_volume.view(),
                shadow_volume.sampler(),
            )?
            .apply(rrdevice);
        self.update_scene_depth(rrdevice, scene_depth_view)
    }

    pub unsafe fn update_scene_depth(
        &self,
        rrdevice: &RRDevice,
        scene_depth_view: ImageView,
    ) -> Result<()> {
        self.sets
            .writer(SetSlot::SINGLE)?
            .sampled_depth(
                wind_resolve::SCENE_DEPTH_SAMPLER,
                scene_depth_view,
                self.scene_depth_sampler,
            )?
            .apply(rrdevice);
        Ok(())
    }
}

pub unsafe fn create_wind_shadow_bake_sets(rrdevice: &RRDevice) -> Result<ReflectedDescriptorSets> {
    let layout_spec = ReflectedLayoutSpec::local(&WIND_SHADOW_BAKE)
        .with_override(wind_shadow_bake::WIND, DescriptorKind::UniformBufferDynamic);
    ReflectedDescriptorSets::create(rrdevice, &layout_spec, SetGrid::SINGLE)
}

pub unsafe fn write_wind_shadow_bake_set(
    sets: &ReflectedDescriptorSets,
    rrdevice: &RRDevice,
    wind_ubo: &UniformBuffer<WindUBO>,
    shadow_volume: &VolumeImage,
) -> Result<()> {
    sets.writer(SetSlot::SINGLE)?
        .uniform_dynamic(wind_shadow_bake::WIND, wind_ubo)?
        .storage_image(wind_shadow_bake::SHADOW_VOLUME_IMAGE, shadow_volume.view())?
        .apply(rrdevice);
    Ok(())
}
