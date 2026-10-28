use anyhow::{anyhow, Result};

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::shader_bindings::water_resolve;
use crate::vulkanr::descriptor::{
    DescriptorKind, ReflectedLayoutSpec, ReflectedSetLayout, WATER_RESOLVE,
};
use thyllore_effect_core::WaterUBO;
use thyllore_vulkan_core::resource::{GpuResource, UniformBuffer};
use thyllore_vulkan_core::types::{
    AccelerationStructure, Buffer, DescriptorSet, ImageView, Sampler,
};

const WATER_HISTORY_SET_COUNT: usize = 2;

#[derive(Clone, Debug, Default)]
pub struct RRWaterDescriptorSet {
    pub layout: ReflectedSetLayout,
    frames: Vec<[DescriptorSet; WATER_HISTORY_SET_COUNT]>,
}

impl RRWaterDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WATER_RESOLVE).with_override(
            water_resolve::WATER_BLOCK,
            DescriptorKind::UniformBufferDynamic,
        )
    }

    pub unsafe fn new(rrdevice: &RRDevice, frames_in_flight: usize) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;

        let mut frames = Vec::with_capacity(frames_in_flight.max(1));
        for _ in 0..frames_in_flight.max(1) {
            frames.push([
                layout.allocate_descriptor_set(rrdevice)?,
                layout.allocate_descriptor_set(rrdevice)?,
            ]);
        }

        Ok(Self { layout, frames })
    }

    pub fn descriptor_set(&self, frame_slot: usize, history_index: usize) -> Result<DescriptorSet> {
        let sets = self.frames.get(frame_slot).ok_or_else(|| {
            anyhow!(
                "water descriptor slot {frame_slot} exceeds {} frames",
                self.frames.len()
            )
        })?;
        sets.get(history_index)
            .copied()
            .ok_or_else(|| anyhow!("water history index {history_index} is out of range"))
    }

    pub unsafe fn write_all_at(
        &self,
        rrdevice: &RRDevice,
        frame_slot: usize,
        water_ubo: &UniformBuffer<WaterUBO>,
        scene_color_view: ImageView,
        scene_color_sampler: Sampler,
        history_image_views: [ImageView; 2],
        history_sampler: Sampler,
        trace_image_view: ImageView,
        trace_sampler: Sampler,
        tlas: AccelerationStructure,
        hit_table: Buffer,
    ) -> Result<()> {
        for i in 0..WATER_HISTORY_SET_COUNT {
            let descriptor_set = self.descriptor_set(frame_slot, i)?;
            let previous_history_view = history_image_views[1 - i];
            self.layout
                .descriptor_writer(descriptor_set)
                .uniform_dynamic(water_resolve::WATER_BLOCK, water_ubo)?
                .sampled_image(
                    water_resolve::SCENE_COLOR_SAMPLER,
                    scene_color_view,
                    scene_color_sampler,
                )?
                .sampled_image(
                    water_resolve::WATER_HISTORY_SAMPLER,
                    previous_history_view,
                    history_sampler,
                )?
                .sampled_storage_image(
                    water_resolve::WATER_TRACE_SAMPLER,
                    trace_image_view,
                    trace_sampler,
                )?
                .acceleration_structure(water_resolve::SCENE_TLAS, tlas)?
                .whole_buffer(water_resolve::HIT_TABLE, hit_table)?
                .apply(rrdevice);
        }
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.layout.destroy(rrdevice);
    }
}

impl GpuResource for RRWaterDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.layout.is_destroyed()
    }
}
