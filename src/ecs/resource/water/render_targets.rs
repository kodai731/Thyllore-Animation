use thyllore_effect_core::WaterUBO;
use thyllore_vulkan_core::pipeline::RRPipeline;
use thyllore_vulkan_core::resource::hdr_buffer::HDR_FORMAT;
use thyllore_vulkan_core::resource::render_target_transient::TransientDesc;
use thyllore_vulkan_core::resource::{
    AccumulationTarget, GpuResource, HistoryTargets, UniformBuffer,
};
use vulkanalia::prelude::v1_0::*;

use crate::ecs::systems::water::{RRWaterCausticDescriptorSet, RRWaterDescriptorSet};
use crate::gpu_resource;

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
    pub history: HistoryTargets,
    pub caustic_accum: AccumulationTarget,
    #[gpu_resource(skip)]
    pub frame_instances: Vec<(thyllore_effect_core::WaterUBO, u32)>,
    #[gpu_resource(skip)]
    bound: Vec<Option<WaterBindingKey>>,
}

gpu_resource!(WaterRenderTargets);

#[derive(Debug, Default, GpuResource)]
pub struct WaterGpuState {
    pub shading_pipeline: Option<RRPipeline>,
    pub descriptor: Option<RRWaterDescriptorSet>,
    pub ubo: Option<UniformBuffer<WaterUBO>>,
    pub caustic_splat_pipeline: Option<RRPipeline>,
    pub caustic_apply_pipeline: Option<RRPipeline>,
    pub caustic_descriptor: Option<RRWaterCausticDescriptorSet>,
    #[gpu_resource(skip)]
    pub caustic_bound_tlas: vk::AccelerationStructureKHR,
}

gpu_resource!(WaterGpuState);

impl WaterRenderTargets {
    pub fn new(history: HistoryTargets, caustic_accum: AccumulationTarget) -> Self {
        Self {
            history,
            caustic_accum,
            frame_instances: Vec::new(),
            bound: Vec::new(),
        }
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.history.extent()
    }

    pub fn scene_color_desc(&self) -> TransientDesc {
        TransientDesc {
            width: self.history.width,
            height: self.history.height,
            format: HDR_FORMAT,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        }
    }

    pub fn trace_desc(&self) -> TransientDesc {
        TransientDesc {
            width: self.history.width,
            height: self.history.height,
            format: vk::Format::R16G16B16A16_SFLOAT,
            usage: vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
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
