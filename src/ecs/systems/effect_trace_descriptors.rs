use crate::vulkanr::core::*;
use crate::vulkanr::descriptor::pass_manifest::EFFECT_TRACE;
use crate::vulkanr::descriptor::shader_bindings::effect_trace;
use crate::vulkanr::descriptor::{
    ReflectedDescriptorSets, ReflectedLayoutSpec, ReflectedSetLayout,
};
use crate::vulkanr::resource::GpuResource;
use thyllore_vulkan_core::vulkan::*;

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

    pub fn descriptor_set(&self, frame_slot: usize) -> Result<vk::DescriptorSet> {
        if frame_slot >= self.sets.len() {
            anyhow::bail!(
                "effect trace descriptor slot {frame_slot} exceeds {} sets",
                self.sets.len()
            );
        }
        Ok(self.sets.set(frame_slot))
    }

    pub unsafe fn write_all_at(
        &self,
        rrdevice: &RRDevice,
        frame_slot: usize,
        tlas: vk::AccelerationStructureKHR,
        trace_image_view: vk::ImageView,
        hit_table: vk::Buffer,
    ) -> Result<()> {
        self.sets
            .writer(frame_slot)
            .acceleration_structure(effect_trace::TLAS, tlas)?
            .image(
                effect_trace::OUT_IMAGE,
                trace_image_view,
                vk::Sampler::null(),
                vk::ImageLayout::GENERAL,
            )?
            .buffer(effect_trace::HIT_TABLE, hit_table, 0, vk::WHOLE_SIZE as u64)?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        self.sets.destroy(device);
    }
}

impl GpuResource for RREffectTraceDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device);
    }
}
