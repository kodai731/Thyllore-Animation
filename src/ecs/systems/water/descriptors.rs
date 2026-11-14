use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::shader_bindings::water_resolve;
use crate::vulkanr::descriptor::{
    DescriptorKind, ReflectedDescriptorSets, ReflectedLayoutSpec, SetGrid, SetSlot, WATER_RESOLVE,
};
use thyllore_effect_core::WaterUBO;
use thyllore_vulkan_core::resource::UniformBuffer;
use thyllore_vulkan_core::types::{AccelerationStructure, Buffer, ImageView, Sampler};

const WATER_HISTORY_SET_COUNT: usize = 2;

#[derive(Clone, Copy, Debug)]
pub struct WaterResolveBindings {
    pub scene_color_view: ImageView,
    pub history_image_views: [ImageView; WATER_HISTORY_SET_COUNT],
    pub trace_image_view: ImageView,
    pub sampler: Sampler,
    pub tlas: AccelerationStructure,
    pub hit_table: Buffer,
}

pub unsafe fn create_water_resolve_sets(
    rrdevice: &RRDevice,
    frames_in_flight: usize,
) -> Result<ReflectedDescriptorSets> {
    let layout_spec = ReflectedLayoutSpec::local(&WATER_RESOLVE).with_override(
        water_resolve::WATER_BLOCK,
        DescriptorKind::UniformBufferDynamic,
    );
    ReflectedDescriptorSets::create(
        rrdevice,
        &layout_spec,
        SetGrid::new(frames_in_flight, WATER_HISTORY_SET_COUNT),
    )
}

/// Writes both history sets of a frame slot; each samples the history image the other index wrote.
pub unsafe fn write_water_resolve_sets(
    sets: &ReflectedDescriptorSets,
    rrdevice: &RRDevice,
    frame_slot: usize,
    water_ubo: &UniformBuffer<WaterUBO>,
    bindings: WaterResolveBindings,
) -> Result<()> {
    for history_index in 0..WATER_HISTORY_SET_COUNT {
        let previous_history_view = bindings.history_image_views[1 - history_index];
        sets.writer(SetSlot::new(frame_slot, history_index))?
            .uniform_dynamic(water_resolve::WATER_BLOCK, water_ubo)?
            .sampled_image(
                water_resolve::SCENE_COLOR_SAMPLER,
                bindings.scene_color_view,
                bindings.sampler,
            )?
            .sampled_image(
                water_resolve::WATER_HISTORY_SAMPLER,
                previous_history_view,
                bindings.sampler,
            )?
            .sampled_storage_image(
                water_resolve::WATER_TRACE_SAMPLER,
                bindings.trace_image_view,
                bindings.sampler,
            )?
            .acceleration_structure(water_resolve::SCENE_TLAS, bindings.tlas)?
            .whole_buffer(water_resolve::HIT_TABLE, bindings.hit_table)?
            .apply(rrdevice);
    }
    Ok(())
}
