use std::collections::HashMap;

use imgui::SnapshotTextureId;
use vulkanalia::prelude::v1_0::*;

use crate::gpu_resource;
use crate::vulkanr::resource::{GpuResource, SampledTexture};

/// The textures Dear ImGui asked the renderer to own; a draw command carries the
/// descriptor set handle of one of them as its texture id.
#[must_use]
#[derive(Debug, Default, GpuResource)]
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

gpu_resource!(ImguiTextures);
