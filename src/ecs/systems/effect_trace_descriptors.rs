use crate::vulkanr::core::*;
use crate::vulkanr::descriptor::pass_manifest::EFFECT_TRACE;
use crate::vulkanr::descriptor::shader_bindings::effect_trace;
use crate::vulkanr::descriptor::{ReflectedLayoutSpec, ReflectedSetLayout};
use crate::vulkanr::resource::GpuResource;
use thyllore_vulkan_core::vulkan::*;

#[derive(Clone, Debug, Default)]
pub struct RREffectTraceDescriptorSet {
    pub layout: ReflectedSetLayout,
    descriptor_sets: Vec<vk::DescriptorSet>,
}

impl RREffectTraceDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&EFFECT_TRACE)
    }

    pub unsafe fn new(rrdevice: &RRDevice, frames_in_flight: usize) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let descriptor_sets = layout.allocate_sets(rrdevice, frames_in_flight.max(1))?;

        Ok(Self {
            layout,
            descriptor_sets,
        })
    }

    pub fn descriptor_set(&self, frame_slot: usize) -> Result<vk::DescriptorSet> {
        self.descriptor_sets
            .get(frame_slot)
            .copied()
            .ok_or_else(|| {
                anyhow!(
                    "effect trace descriptor slot {frame_slot} exceeds {} sets",
                    self.descriptor_sets.len()
                )
            })
    }

    pub unsafe fn write_all_at(
        &self,
        rrdevice: &RRDevice,
        frame_slot: usize,
        tlas: vk::AccelerationStructureKHR,
        trace_image_view: vk::ImageView,
        hit_table: vk::Buffer,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set(frame_slot)?)
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
        self.layout.destroy(device);
    }
}

impl GpuResource for RREffectTraceDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device);
    }
}
