use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::data::SceneUniformData;
use crate::vulkanr::descriptor::shader_bindings::{water_caustic_apply, water_caustic_splat};
use crate::vulkanr::descriptor::{
    ReflectedDescriptorSets, ReflectedLayoutSpec, SetGrid, SetSlot, WATER_CAUSTIC_APPLY,
    WATER_CAUSTIC_SPLAT,
};
use thyllore_vulkan_core::types::{AccelerationStructure, Buffer, ImageView};

const SCENE_DATA_SIZE: u64 = std::mem::size_of::<SceneUniformData>() as u64;

#[derive(Clone, Copy, Debug)]
pub struct WaterCausticBindings {
    pub caustic_accum_view: ImageView,
    pub position_image_view: ImageView,
    pub tlas: Option<AccelerationStructure>,
    pub scene_uniform_buffer: Buffer,
    pub water_ubo: Buffer,
    pub hdr_color_image_view: ImageView,
}

pub unsafe fn create_water_caustic_splat_sets(
    rrdevice: &RRDevice,
) -> Result<ReflectedDescriptorSets> {
    ReflectedDescriptorSets::create(
        rrdevice,
        &ReflectedLayoutSpec::local(&WATER_CAUSTIC_SPLAT),
        SetGrid::SINGLE,
    )
}

pub unsafe fn create_water_caustic_apply_sets(
    rrdevice: &RRDevice,
) -> Result<ReflectedDescriptorSets> {
    ReflectedDescriptorSets::create(
        rrdevice,
        &ReflectedLayoutSpec::local(&WATER_CAUSTIC_APPLY),
        SetGrid::SINGLE,
    )
}

pub unsafe fn write_water_caustic_splat_set(
    sets: &ReflectedDescriptorSets,
    rrdevice: &RRDevice,
    bindings: WaterCausticBindings,
) -> Result<()> {
    let writer = sets
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
            SCENE_DATA_SIZE,
        )?
        .whole_buffer(water_caustic_splat::WATER_BLOCK, bindings.water_ubo)?;
    let writer = match bindings.tlas {
        Some(tlas) => writer.acceleration_structure(water_caustic_splat::TOP_LEVEL_AS, tlas)?,
        None => writer,
    };

    writer.apply(rrdevice);
    Ok(())
}

pub unsafe fn update_water_caustic_splat_tlas(
    sets: &ReflectedDescriptorSets,
    rrdevice: &RRDevice,
    tlas: AccelerationStructure,
) -> Result<()> {
    sets.writer(SetSlot::SINGLE)?
        .acceleration_structure(water_caustic_splat::TOP_LEVEL_AS, tlas)?
        .apply(rrdevice);
    Ok(())
}

pub unsafe fn write_water_caustic_apply_set(
    sets: &ReflectedDescriptorSets,
    rrdevice: &RRDevice,
    bindings: WaterCausticBindings,
) -> Result<()> {
    sets.writer(SetSlot::SINGLE)?
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
            SCENE_DATA_SIZE,
        )?
        .whole_buffer(water_caustic_apply::WATER_BLOCK, bindings.water_ubo)?
        .apply(rrdevice);
    Ok(())
}
