use std::collections::HashMap;

use imgui::SnapshotTextureId;
use vulkanalia::prelude::v1_0::*;

use crate::gpu_resource;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::resource::{GpuResource, SampledTexture};

/// The textures Dear ImGui asked the renderer to own; a draw command carries the
/// descriptor set handle of one of them as its texture id.
#[derive(Clone, Debug, Default)]
pub struct ImguiTextures {
    pub textures: HashMap<SnapshotTextureId, SampledTexture>,
}

impl ImguiTextures {
    pub fn descriptor_set_for(&self, texture_id: u64) -> Option<vk::DescriptorSet> {
        self.textures
            .values()
            .map(SampledTexture::descriptor_set)
            .find(|set| set.as_raw() == texture_id)
    }
}

impl GpuResource for ImguiTextures {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        for (_, mut texture) in self.textures.drain() {
            texture.destroy_gpu(rrdevice);
        }
    }
}

gpu_resource!(ImguiTextures);
