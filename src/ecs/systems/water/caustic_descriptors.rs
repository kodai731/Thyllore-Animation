use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::data::SceneUniformData;
use crate::vulkanr::descriptor::shader_bindings::{water_caustic_apply, water_caustic_splat};
use crate::vulkanr::descriptor::{
    ReflectedLayoutSpec, ReflectedSetLayout, WATER_CAUSTIC_APPLY, WATER_CAUSTIC_SPLAT,
};
use thyllore_vulkan_core::resource::GpuResource;
use thyllore_vulkan_core::types::{AccelerationStructure, Buffer, DescriptorSet, ImageView};

#[derive(Clone, Debug, Default)]
pub struct RRWaterCausticDescriptorSet {
    pub splat_layout: ReflectedSetLayout,
    pub splat_descriptor_set: DescriptorSet,
    pub apply_layout: ReflectedSetLayout,
    pub apply_descriptor_set: DescriptorSet,
}

impl RRWaterCausticDescriptorSet {
    pub fn splat_layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WATER_CAUSTIC_SPLAT)
    }

    pub fn apply_layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WATER_CAUSTIC_APPLY)
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let splat_layout = ReflectedSetLayout::create(rrdevice, &Self::splat_layout_spec())?;
        let apply_layout = ReflectedSetLayout::create(rrdevice, &Self::apply_layout_spec())?;

        Ok(Self {
            splat_layout,
            splat_descriptor_set: DescriptorSet::default(),
            apply_layout,
            apply_descriptor_set: DescriptorSet::default(),
        })
    }

    pub fn is_allocated(&self) -> bool {
        self.splat_descriptor_set != DescriptorSet::default()
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
        if !self.is_allocated() {
            self.splat_descriptor_set = self.splat_layout.allocate_descriptor_set(rrdevice)?;
        }
        if self.apply_descriptor_set == DescriptorSet::default() {
            self.apply_descriptor_set = self.apply_layout.allocate_descriptor_set(rrdevice)?;
        }

        let mut splat_writer = self
            .splat_layout
            .descriptor_writer(self.splat_descriptor_set)
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

        self.apply_layout
            .descriptor_writer(self.apply_descriptor_set)
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
        if !self.is_allocated() {
            return Ok(());
        }
        self.splat_layout
            .descriptor_writer(self.splat_descriptor_set)
            .acceleration_structure(water_caustic_splat::TOP_LEVEL_AS, tlas)?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.splat_layout.destroy(rrdevice);
        self.apply_layout.destroy(rrdevice);
    }
}

impl GpuResource for RRWaterCausticDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.splat_layout.is_destroyed() && self.apply_layout.is_destroyed()
    }
}
