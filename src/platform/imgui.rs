use imgui::SynchronousRendererConsumer;
use vulkanalia::prelude::v1_0::*;

use crate::app::init::MAX_FRAMES_IN_FLIGHT;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::ReflectedSetLayout;
use crate::vulkanr::resource::GpuResource;

#[derive(Debug, Default)]
pub struct ImguiData {
    pub pipeline: Option<vk::Pipeline>,
    pub pipeline_layout: Option<vk::PipelineLayout>,
    pub descriptor_set_layout: Option<ReflectedSetLayout>,
    pub sampler: Option<vk::Sampler>,
    pub renderer_consumer: Option<SynchronousRendererConsumer>,
    pub vertex_buffers: [Option<vk::Buffer>; MAX_FRAMES_IN_FLIGHT],
    pub vertex_buffer_memories: [Option<vk::DeviceMemory>; MAX_FRAMES_IN_FLIGHT],
    pub vertex_buffer_sizes: [vk::DeviceSize; MAX_FRAMES_IN_FLIGHT],
    pub index_buffers: [Option<vk::Buffer>; MAX_FRAMES_IN_FLIGHT],
    pub index_buffer_memories: [Option<vk::DeviceMemory>; MAX_FRAMES_IN_FLIGHT],
    pub index_buffer_sizes: [vk::DeviceSize; MAX_FRAMES_IN_FLIGHT],
}

impl ImguiData {
    unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        let device = &rrdevice.device;

        for slot in 0..MAX_FRAMES_IN_FLIGHT {
            destroy_buffer_with_memory(
                device,
                self.vertex_buffers[slot].take(),
                self.vertex_buffer_memories[slot].take(),
            );
            destroy_buffer_with_memory(
                device,
                self.index_buffers[slot].take(),
                self.index_buffer_memories[slot].take(),
            );
            self.vertex_buffer_sizes[slot] = 0;
            self.index_buffer_sizes[slot] = 0;
        }

        if let Some(pipeline) = self.pipeline.take() {
            device.destroy_pipeline(pipeline, None);
        }
        if let Some(pipeline_layout) = self.pipeline_layout.take() {
            device.destroy_pipeline_layout(pipeline_layout, None);
        }
        self.renderer_consumer = None;
        self.descriptor_set_layout.destroy_gpu(rrdevice);

        if let Some(sampler) = self.sampler.take() {
            device.destroy_sampler(sampler, None);
        }
    }
}

unsafe fn destroy_buffer_with_memory(
    device: &vulkanalia::Device,
    buffer: Option<vk::Buffer>,
    memory: Option<vk::DeviceMemory>,
) {
    if let Some(buffer) = buffer {
        device.destroy_buffer(buffer, None);
    }
    if let Some(memory) = memory {
        device.free_memory(memory, None);
    }
}

impl GpuResource for ImguiData {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.pipeline.is_destroyed()
            && self.descriptor_set_layout.is_destroyed()
            && self.sampler.is_destroyed()
            && self.vertex_buffers.iter().all(Option::is_none)
            && self.index_buffers.iter().all(Option::is_none)
    }
}
