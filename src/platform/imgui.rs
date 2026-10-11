use std::collections::HashMap;

use imgui::{SnapshotTextureId, SynchronousRendererConsumer};
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
    pub textures: HashMap<SnapshotTextureId, ImguiTexture>,
    pub renderer_consumer: Option<SynchronousRendererConsumer>,
    pub vertex_buffers: [Option<vk::Buffer>; MAX_FRAMES_IN_FLIGHT],
    pub vertex_buffer_memories: [Option<vk::DeviceMemory>; MAX_FRAMES_IN_FLIGHT],
    pub vertex_buffer_sizes: [vk::DeviceSize; MAX_FRAMES_IN_FLIGHT],
    pub index_buffers: [Option<vk::Buffer>; MAX_FRAMES_IN_FLIGHT],
    pub index_buffer_memories: [Option<vk::DeviceMemory>; MAX_FRAMES_IN_FLIGHT],
    pub index_buffer_sizes: [vk::DeviceSize; MAX_FRAMES_IN_FLIGHT],
}

/// One texture Dear ImGui asked the renderer to own; its draw commands carry the
/// descriptor set handle as the texture id.
#[derive(Clone, Copy, Debug)]
pub struct ImguiTexture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub descriptor_set: vk::DescriptorSet,
}

impl ImguiTexture {
    pub fn texture_id(&self) -> u64 {
        self.descriptor_set.as_raw()
    }

    unsafe fn destroy(self, device: &vulkanalia::Device) {
        device.destroy_image_view(self.view, None);
        device.destroy_image(self.image, None);
        device.free_memory(self.memory, None);
    }
}

impl ImguiData {
    pub fn descriptor_set_for(&self, texture_id: u64) -> Option<vk::DescriptorSet> {
        self.textures
            .values()
            .find(|texture| texture.texture_id() == texture_id)
            .map(|texture| texture.descriptor_set)
    }

    pub unsafe fn destroy_texture(
        &mut self,
        device: &vulkanalia::Device,
        id: SnapshotTextureId,
    ) -> bool {
        match self.textures.remove(&id) {
            Some(texture) => {
                texture.destroy(device);
                true
            }
            None => false,
        }
    }

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
        for (_, texture) in self.textures.drain() {
            texture.destroy(device);
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
}
