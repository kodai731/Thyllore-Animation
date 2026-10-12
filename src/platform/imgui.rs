use imgui::SynchronousRendererConsumer;
use vulkanalia::prelude::v1_0::*;

use crate::app::init::MAX_FRAMES_IN_FLIGHT;
use crate::vulkanr::descriptor::ReflectedSetLayout;
use crate::vulkanr::resource::{BufferAllocation, GpuResource};

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct ImguiData {
    pub sampler: Option<vk::Sampler>,
    pub descriptor_set_layout: Option<ReflectedSetLayout>,
    #[gpu_resource(skip)]
    pub renderer_consumer: Option<SynchronousRendererConsumer>,
    pub pipeline_layout: Option<vk::PipelineLayout>,
    pub pipeline: Option<vk::Pipeline>,
    pub index_buffers: [Option<BufferAllocation>; MAX_FRAMES_IN_FLIGHT],
    #[gpu_resource(skip)]
    pub index_buffer_sizes: [vk::DeviceSize; MAX_FRAMES_IN_FLIGHT],
    pub vertex_buffers: [Option<BufferAllocation>; MAX_FRAMES_IN_FLIGHT],
    #[gpu_resource(skip)]
    pub vertex_buffer_sizes: [vk::DeviceSize; MAX_FRAMES_IN_FLIGHT],
}
