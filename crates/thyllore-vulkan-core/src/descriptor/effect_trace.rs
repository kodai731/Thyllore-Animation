use anyhow::Result;

use crate::core::device::RRDevice;
use crate::descriptor::pass_manifest::EFFECT_TRACE;
use crate::descriptor::shader_bindings::effect_trace;
use crate::descriptor::{ReflectedDescriptorSets, ReflectedLayoutSpec, SetGrid, SetSlot};

pub unsafe fn create_effect_trace_sets(
    rrdevice: &RRDevice,
    frames_in_flight: usize,
) -> Result<ReflectedDescriptorSets> {
    ReflectedDescriptorSets::create(
        rrdevice,
        &ReflectedLayoutSpec::local(&EFFECT_TRACE),
        SetGrid::per_frame_slot(frames_in_flight),
    )
}

pub unsafe fn write_effect_trace_set(
    sets: &ReflectedDescriptorSets,
    rrdevice: &RRDevice,
    frame_slot: usize,
    tlas: crate::types::AccelerationStructure,
    trace_image_view: crate::types::ImageView,
    hit_table: crate::types::Buffer,
) -> Result<()> {
    sets.writer(SetSlot::frame_slot(frame_slot))?
        .acceleration_structure(effect_trace::TLAS, tlas)?
        .storage_image(effect_trace::OUT_IMAGE, trace_image_view)?
        .whole_buffer(effect_trace::HIT_TABLE, hit_table)?
        .apply(rrdevice);
    Ok(())
}
