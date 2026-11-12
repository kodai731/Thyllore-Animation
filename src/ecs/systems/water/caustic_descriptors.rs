use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::data::SceneUniformData;
use crate::vulkanr::descriptor::shader_bindings::{water_caustic_apply, water_caustic_splat};
use crate::vulkanr::descriptor::{
    ReflectedDescriptorSets, ReflectedLayoutSpec, ReflectedSetLayout, WATER_CAUSTIC_APPLY,
    WATER_CAUSTIC_SPLAT,
};
use thyllore_vulkan_core::resource::GpuResource;
use thyllore_vulkan_core::types::{AccelerationStructure, Buffer, DescriptorSet, ImageView};

#[derive(Clone, Debug, Default)]
pub struct RRWaterCausticDescriptorSet {
    splat: ReflectedDescriptorSets,
    apply: ReflectedDescriptorSets,
}

impl RRWaterCausticDescriptorSet {
    pub fn splat_layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WATER_CAUSTIC_SPLAT)
    }

    pub fn apply_layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WATER_CAUSTIC_APPLY)
    }

    pub fn splat_layout(&self) -> &ReflectedSetLayout {
        self.splat.layout()
    }

    pub fn splat_descriptor_set(&self) -> DescriptorSet {
        self.splat.set(0)
    }

    pub fn apply_layout(&self) -> &ReflectedSetLayout {
        self.apply.layout()
    }

    pub fn apply_descriptor_set(&self) -> DescriptorSet {
        self.apply.set(0)
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let splat = ReflectedDescriptorSets::create(rrdevice, &Self::splat_layout_spec(), 1)?;
        let apply = ReflectedDescriptorSets::create(rrdevice, &Self::apply_layout_spec(), 1)?;

        Ok(Self { splat, apply })
    }

    pub unsafe fn allocate_and_update(
        &mut self,
        rrdevice: &RRDevice,
        caustic_accum_view: ImageView,
        position_image_view: ImageView,
        tlas: Option<AccelerationStructure>,
        scene_uniform_buffer: Buffer,
        water_ubo: Buffer,
        hdr_color_image_view: ImageView,
    ) -> Result<()> {
        let mut splat_writer = self
            .splat
            .writer(0)
            .storage_image(water_caustic_splat::CAUSTIC_ACCUM_IMAGE, caustic_accum_view)?
            .storage_image(water_caustic_splat::POSITION_IMAGE, position_image_view)?;
        if let Some(tlas) = tlas {
            splat_writer =
                splat_writer.acceleration_structure(water_caustic_splat::TOP_LEVEL_AS, tlas)?;
        }
        splat_writer
            .buffer_range(
                water_caustic_splat::SCENE_DATA,
                scene_uniform_buffer,
                0,
                std::mem::size_of::<SceneUniformData>() as u64,
            )?
            .whole_buffer(water_caustic_splat::WATER_BLOCK, water_ubo)?
            .apply(rrdevice);

        self.apply
            .writer(0)
            .storage_image(water_caustic_apply::CAUSTIC_ACCUM_IMAGE, caustic_accum_view)?
            .storage_image(water_caustic_apply::HDR_COLOR_IMAGE, hdr_color_image_view)?
            .buffer_range(
                water_caustic_apply::SCENE_DATA,
                scene_uniform_buffer,
                0,
                std::mem::size_of::<SceneUniformData>() as u64,
            )?
            .whole_buffer(water_caustic_apply::WATER_BLOCK, water_ubo)?
            .apply(rrdevice);

        Ok(())
    }

    pub unsafe fn update_tlas(
        &mut self,
        rrdevice: &RRDevice,
        tlas: AccelerationStructure,
    ) -> Result<()> {
        self.splat
            .writer(0)
            .acceleration_structure(water_caustic_splat::TOP_LEVEL_AS, tlas)?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.splat.destroy(rrdevice);
        self.apply.destroy(rrdevice);
    }
}

impl GpuResource for RRWaterCausticDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.splat.is_destroyed() && self.apply.is_destroyed()
    }
}
