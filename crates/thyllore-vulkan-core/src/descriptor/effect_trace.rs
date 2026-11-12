use crate::core::*;
use crate::descriptor::pass_manifest::EFFECT_TRACE;
use crate::descriptor::shader_bindings::effect_trace;
use crate::descriptor::{ReflectedDescriptorSets, ReflectedLayoutSpec, ReflectedSetLayout};
use crate::resource::GpuResource;
use crate::vulkan::*;

#[derive(Clone, Debug, Default)]
pub struct RREffectTraceDescriptorSet {
    sets: ReflectedDescriptorSets,
}

impl RREffectTraceDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&EFFECT_TRACE)
    }

    pub unsafe fn new(rrdevice: &RRDevice, frames_in_flight: usize) -> Result<Self> {
        let sets =
            ReflectedDescriptorSets::create(rrdevice, &Self::layout_spec(), frames_in_flight)?;

        Ok(Self { sets })
    }

    pub fn layout(&self) -> &ReflectedSetLayout {
        self.sets.layout()
    }

    pub fn descriptor_set(&self, frame_slot: usize) -> Result<crate::types::DescriptorSet> {
        self.sets.get(frame_slot).ok_or_else(|| {
            anyhow!(
                "effect trace descriptor slot {frame_slot} exceeds {} sets",
                self.sets.len()
            )
        })
    }

    pub unsafe fn write_all_at(
        &self,
        rrdevice: &RRDevice,
        frame_slot: usize,
        tlas: crate::types::AccelerationStructure,
        trace_image_view: crate::types::ImageView,
        hit_table: crate::types::Buffer,
    ) -> Result<()> {
        self.sets
            .writer(frame_slot)
            .acceleration_structure(effect_trace::TLAS, tlas)?
            .storage_image(effect_trace::OUT_IMAGE, trace_image_view)?
            .buffer(
                effect_trace::HIT_TABLE,
                hit_table.raw(),
                0,
                vk::WHOLE_SIZE as u64,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.sets.destroy(rrdevice);
    }
}

impl GpuResource for RREffectTraceDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.sets.is_destroyed()
    }
}
