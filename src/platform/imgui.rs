use vulkanalia::prelude::v1_0::*;

use crate::app::init::MAX_FRAMES_IN_FLIGHT;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::ReflectedSetLayout;
use crate::vulkanr::resource::GpuResource;

#[derive(Clone, Debug)]
pub struct ImguiData {
    pub pipeline: Option<vk::Pipeline>,
    pub pipeline_layout: Option<vk::PipelineLayout>,
    pub descriptor_set: Option<vk::DescriptorSet>,
    pub descriptor_set_layout: Option<ReflectedSetLayout>,
    pub font_image: Option<vk::Image>,
    pub font_image_memory: Option<vk::DeviceMemory>,
    pub font_image_view: Option<vk::ImageView>,
    pub sampler: Option<vk::Sampler>,
    pub vertex_buffers: [Option<vk::Buffer>; MAX_FRAMES_IN_FLIGHT],
    pub vertex_buffer_memories: [Option<vk::DeviceMemory>; MAX_FRAMES_IN_FLIGHT],
    pub vertex_buffer_sizes: [vk::DeviceSize; MAX_FRAMES_IN_FLIGHT],
    pub index_buffers: [Option<vk::Buffer>; MAX_FRAMES_IN_FLIGHT],
    pub index_buffer_memories: [Option<vk::DeviceMemory>; MAX_FRAMES_IN_FLIGHT],
    pub index_buffer_sizes: [vk::DeviceSize; MAX_FRAMES_IN_FLIGHT],
}

impl Default for ImguiData {
    fn default() -> Self {
        Self {
            pipeline: None,
            pipeline_layout: None,
            descriptor_set: None,
            descriptor_set_layout: None,
            font_image: None,
            font_image_memory: None,
            font_image_view: None,
            sampler: None,
            vertex_buffers: [None, None],
            vertex_buffer_memories: [None, None],
            vertex_buffer_sizes: [0, 0],
            index_buffers: [None, None],
            index_buffer_memories: [None, None],
            index_buffer_sizes: [0, 0],
        }
    }
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
        self.descriptor_set = None;
        self.descriptor_set_layout.destroy_gpu(rrdevice);

        if let Some(sampler) = self.sampler.take() {
            device.destroy_sampler(sampler, None);
        }
        if let Some(view) = self.font_image_view.take() {
            device.destroy_image_view(view, None);
        }
        if let Some(image) = self.font_image.take() {
            device.destroy_image(image, None);
        }
        if let Some(memory) = self.font_image_memory.take() {
            device.free_memory(memory, None);
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
}
