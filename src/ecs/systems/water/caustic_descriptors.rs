use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::data::SceneUniformData;
use crate::vulkanr::descriptor::shader_bindings::{water_caustic_apply, water_caustic_splat};
use crate::vulkanr::descriptor::{
    ReflectedDescriptorSets, ReflectedLayoutSpec, SetGrid, SetSlot, WATER_CAUSTIC_APPLY,
    WATER_CAUSTIC_SPLAT,
};
use thyllore_vulkan_core::resource::GpuResource;
use thyllore_vulkan_core::types::{AccelerationStructure, Buffer, ImageView};

#[derive(Clone, Copy, Debug)]
pub struct WaterCausticBindings {
    pub caustic_accum_view: ImageView,
    pub position_image_view: ImageView,
    pub tlas: Option<AccelerationStructure>,
    pub scene_uniform_buffer: Buffer,
    pub water_ubo: Buffer,
    pub hdr_color_image_view: ImageView,
}

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct RRWaterCausticDescriptorSet {
    pub splat: ReflectedDescriptorSets,
    pub apply: ReflectedDescriptorSets,
}

impl RRWaterCausticDescriptorSet {
    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let splat = ReflectedDescriptorSets::create(
            rrdevice,
            &ReflectedLayoutSpec::local(&WATER_CAUSTIC_SPLAT),
            SetGrid::SINGLE,
        )?;
        let apply = ReflectedDescriptorSets::create(
            rrdevice,
            &ReflectedLayoutSpec::local(&WATER_CAUSTIC_APPLY),
            SetGrid::SINGLE,
        )?;

        Ok(Self { splat, apply })
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        bindings: WaterCausticBindings,
    ) -> Result<()> {
        let scene_data_size = std::mem::size_of::<SceneUniformData>() as u64;

        self.splat
            .writer(SetSlot::SINGLE)?
            .storage_image(
                water_caustic_splat::CAUSTIC_ACCUM_IMAGE,
                bindings.caustic_accum_view,
            )?
            .storage_image(
                water_caustic_splat::POSITION_IMAGE,
                bindings.position_image_view,
            )?
            .buffer_range(
                water_caustic_splat::SCENE_DATA,
                bindings.scene_uniform_buffer,
                0,
                scene_data_size,
            )?
            .whole_buffer(water_caustic_splat::WATER_BLOCK, bindings.water_ubo)?
            .apply(rrdevice);
        if let Some(tlas) = bindings.tlas {
            self.update_tlas(rrdevice, tlas)?;
        }

        self.apply
            .writer(SetSlot::SINGLE)?
            .storage_image(
                water_caustic_apply::CAUSTIC_ACCUM_IMAGE,
                bindings.caustic_accum_view,
            )?
            .storage_image(
                water_caustic_apply::HDR_COLOR_IMAGE,
                bindings.hdr_color_image_view,
            )?
            .buffer_range(
                water_caustic_apply::SCENE_DATA,
                bindings.scene_uniform_buffer,
                0,
                scene_data_size,
            )?
            .whole_buffer(water_caustic_apply::WATER_BLOCK, bindings.water_ubo)?
            .apply(rrdevice);

        Ok(())
    }

    pub unsafe fn update_tlas(
        &self,
        rrdevice: &RRDevice,
        tlas: AccelerationStructure,
    ) -> Result<()> {
        self.splat
            .writer(SetSlot::SINGLE)?
            .acceleration_structure(water_caustic_splat::TOP_LEVEL_AS, tlas)?
            .apply(rrdevice);
        Ok(())
    }
}
