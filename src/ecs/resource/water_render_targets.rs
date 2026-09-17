use thyllore_vulkan_core::resource::{GpuResource, WaterBuffer};

use crate::gpu_resource;
use vulkanalia::prelude::v1_0::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WaterBindingKey {
    pub tlas: vk::AccelerationStructureKHR,
    pub hit_table: vk::Buffer,
    pub history_views: [vk::ImageView; 2],
    pub scene_color_generation: u64,
    pub trace_generation: u64,
}

#[derive(Debug, Default, GpuResource)]
pub struct WaterRenderTargets {
    pub buffer: WaterBuffer,
    #[gpu_resource(skip)]
    pub frame_instances: Vec<(thyllore_effect_core::WaterUBO, u32)>,
    #[gpu_resource(skip)]
    bound: Vec<Option<WaterBindingKey>>,
}

gpu_resource!(WaterRenderTargets);

impl WaterRenderTargets {
    pub fn new(buffer: WaterBuffer) -> Self {
        Self {
            buffer,
            frame_instances: Vec::new(),
            bound: Vec::new(),
        }
    }

    pub fn forget_bindings(&mut self) {
        self.bound.clear();
    }

    pub fn is_bound(&self, frame_slot: usize, key: WaterBindingKey) -> bool {
        self.bound.get(frame_slot).copied().flatten() == Some(key)
    }

    pub fn mark_bound(&mut self, frame_slot: usize, key: WaterBindingKey) {
        if self.bound.len() <= frame_slot {
            self.bound.resize(frame_slot + 1, None);
        }
        self.bound[frame_slot] = Some(key);
    }
}
