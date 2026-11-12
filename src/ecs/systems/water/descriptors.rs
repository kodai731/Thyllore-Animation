use anyhow::{anyhow, Result};

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::shader_bindings::water_resolve;
use crate::vulkanr::descriptor::{
    DescriptorKind, ReflectedDescriptorSets, ReflectedLayoutSpec, ReflectedSetLayout, WATER_RESOLVE,
};
use thyllore_effect_core::WaterUBO;
use thyllore_vulkan_core::resource::{GpuResource, UniformBuffer};
use thyllore_vulkan_core::types::{
    AccelerationStructure, Buffer, DescriptorSet, ImageView, Sampler,
};

const WATER_HISTORY_SET_COUNT: usize = 2;

#[derive(Clone, Debug, Default)]
pub struct RRWaterDescriptorSet {
    sets: ReflectedDescriptorSets,
}

impl RRWaterDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WATER_RESOLVE).with_override(
            water_resolve::WATER_BLOCK,
            DescriptorKind::UniformBufferDynamic,
        )
    }

    fn index(frame_slot: usize, history_index: usize) -> usize {
        frame_slot * WATER_HISTORY_SET_COUNT + history_index
    }

    pub fn layout(&self) -> &ReflectedSetLayout {
        self.sets.layout()
    }

    pub unsafe fn new(rrdevice: &RRDevice, frames_in_flight: usize) -> Result<Self> {
        let count = frames_in_flight.max(1) * WATER_HISTORY_SET_COUNT;
        let sets = ReflectedDescriptorSets::create(rrdevice, &Self::layout_spec(), count)?;
        Ok(Self { sets })
    }

    pub fn descriptor_set(&self, frame_slot: usize, history_index: usize) -> Result<DescriptorSet> {
        let idx = Self::index(frame_slot, history_index);
        self.sets.get(idx).ok_or_else(|| {
            anyhow!(
                "water descriptor index {} (frame_slot={}, history_index={}) exceeds {} sets",
                idx,
                frame_slot,
                history_index,
                self.sets.len()
            )
        })
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
            let previous_history_view = history_image_views[1 - i];
            let idx = Self::index(frame_slot, i);
            self.sets
                .writer(idx)
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
        self.sets.destroy(rrdevice);
    }
}

impl GpuResource for RRWaterDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.sets.is_destroyed()
    }
}
