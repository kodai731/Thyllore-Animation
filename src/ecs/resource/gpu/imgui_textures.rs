use std::collections::HashMap;

use imgui::SnapshotTextureId;

use crate::gpu_resource;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::imgui_texture_id;
use crate::vulkanr::resource::{GpuResource, SampledTexture};
use thyllore_vulkan_core::types::DescriptorSet;

/// The textures Dear ImGui asked the renderer to own; a draw command carries the
/// descriptor set handle of one of them as its texture id.
#[derive(Debug, Default)]
pub struct ImguiTextures {
    pub textures: HashMap<SnapshotTextureId, SampledTexture>,
}

impl ImguiTextures {
    pub fn descriptor_set_for(&self, texture_id: u64) -> Option<DescriptorSet> {
        self.textures
            .values()
            .map(SampledTexture::descriptor_set)
            .find(|set| imgui_texture_id(*set) == texture_id)
    }
}

impl GpuResource for ImguiTextures {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        for (_, mut texture) in self.textures.drain() {
            texture.destroy_gpu(rrdevice);
        }
    }

    fn is_destroyed(&self) -> bool {
        self.textures.values().all(GpuResource::is_destroyed)
    }
}

gpu_resource!(ImguiTextures);
