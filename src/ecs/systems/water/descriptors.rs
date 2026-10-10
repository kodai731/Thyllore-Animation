use crate::vulkanr::core::device::*;
use crate::vulkanr::descriptor::pass_manifest::WATER_RESOLVE;
use crate::vulkanr::descriptor::shader_bindings::water_resolve;
use crate::vulkanr::descriptor::{
    ReflectedDescriptorSets, ReflectedLayoutSpec, ReflectedSetLayout,
};
use crate::vulkanr::resource::gpu_resource::GpuResource;
use crate::vulkanr::resource::uniform_buffer::UniformBuffer;
use crate::vulkanr::vulkan::*;
use thyllore_effect_core::WaterUBO;

const WATER_HISTORY_SET_COUNT: usize = 2;

#[derive(Clone, Debug, Default)]
pub struct RRWaterDescriptorSet {
    sets: ReflectedDescriptorSets,
}

impl RRWaterDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&WATER_RESOLVE).with_override(
            water_resolve::WATER_BLOCK,
            vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
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

    pub fn descriptor_set(
        &self,
        frame_slot: usize,
        history_index: usize,
    ) -> Result<vk::DescriptorSet> {
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
        scene_color_view: vk::ImageView,
        scene_color_sampler: vk::Sampler,
        history_image_views: [vk::ImageView; 2],
        history_sampler: vk::Sampler,
        trace_image_view: vk::ImageView,
        trace_sampler: vk::Sampler,
        tlas: vk::AccelerationStructureKHR,
        hit_table: vk::Buffer,
    ) -> Result<()> {
        for i in 0..WATER_HISTORY_SET_COUNT {
            let previous_history_view = history_image_views[1 - i];
            let idx = Self::index(frame_slot, i);
            self.sets
                .writer(idx)
                .uniform_dynamic(water_resolve::WATER_BLOCK, water_ubo)?
                .image(
                    water_resolve::SCENE_COLOR_SAMPLER,
                    scene_color_view,
                    scene_color_sampler,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                )?
                .image(
                    water_resolve::WATER_HISTORY_SAMPLER,
                    previous_history_view,
                    history_sampler,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                )?
                .image(
                    water_resolve::WATER_TRACE_SAMPLER,
                    trace_image_view,
                    trace_sampler,
                    vk::ImageLayout::GENERAL,
                )?
                .acceleration_structure(water_resolve::SCENE_TLAS, tlas)?
                .buffer(
                    water_resolve::HIT_TABLE,
                    hit_table,
                    0,
                    vk::WHOLE_SIZE as u64,
                )?
                .apply(rrdevice);
        }
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        self.sets.destroy(device);
    }
}

impl GpuResource for RRWaterDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device);
    }
}
