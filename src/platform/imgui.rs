use imgui::SynchronousRendererConsumer;

use crate::app::init::MAX_FRAMES_IN_FLIGHT;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{destroy_imgui_pipeline_objects, ReflectedSetLayout};
use crate::vulkanr::resource::GpuResource;
use thyllore_vulkan_core::resource::HostBuffer;
use thyllore_vulkan_core::types::{Pipeline, PipelineLayout, Sampler};

#[derive(Debug, Default)]
pub struct ImguiData {
    pub pipeline: Option<Pipeline>,
    pub pipeline_layout: Option<PipelineLayout>,
    pub descriptor_set_layout: Option<ReflectedSetLayout>,
    pub sampler: Option<Sampler>,
    pub renderer_consumer: Option<SynchronousRendererConsumer>,
    pub vertex_buffers: [Option<HostBuffer>; MAX_FRAMES_IN_FLIGHT],
    pub index_buffers: [Option<HostBuffer>; MAX_FRAMES_IN_FLIGHT],
}

impl ImguiData {
    unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        for slot in 0..MAX_FRAMES_IN_FLIGHT {
            if let Some(ref mut vb) = self.vertex_buffers[slot] {
                vb.destroy(rrdevice);
            }
            self.vertex_buffers[slot] = None;
            if let Some(ref mut ib) = self.index_buffers[slot] {
                ib.destroy(rrdevice);
            }
            self.index_buffers[slot] = None;
        }

        destroy_imgui_pipeline_objects(
            rrdevice,
            self.pipeline.take(),
            self.pipeline_layout.take(),
            self.sampler.take(),
        );

        self.renderer_consumer = None;
        self.descriptor_set_layout.destroy_gpu(rrdevice);
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
