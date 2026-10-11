use std::collections::HashMap;

use imgui::SnapshotTextureId;
use vulkanalia::prelude::v1_0::*;

use crate::gpu_resource;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::resource::GpuResource;

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

#[derive(Clone, Debug, Default)]
pub struct ImguiTextures {
    pub textures: HashMap<SnapshotTextureId, ImguiTexture>,
}

impl ImguiTextures {
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
}

impl GpuResource for ImguiTextures {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        let device = &rrdevice.device;
        for (_, texture) in self.textures.drain() {
            texture.destroy(device);
        }
    }

    fn resource_name(&self) -> &'static str {
        "ImguiTextures"
    }
}

gpu_resource!(ImguiTextures);
