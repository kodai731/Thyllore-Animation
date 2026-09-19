use std::rc::Rc;

use crate::app::{App, AppData};
use crate::ecs::PassContext;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::core::device::RRDevice;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use crate::vulkanr::resource::GpuBufferRegistry;
use crate::vulkanr::vulkan::Instance;
use crate::vulkanr::VulkanBackend;
use thyllore_vulkan_core::resource::raytracing_data::RayTracingData;
use thyllore_vulkan_core::FrameRenderContext;

pub fn build_frame_render_context(app: &App, image_index: usize) -> FrameRenderContext<'_> {
    FrameRenderContext {
        device: &app.rrdevice,
        graphics: &app.data.graphics_resources,
        buffers: &app.data.buffer_registry,
        pipelines: &app.data.pipeline_storage,
        image_index,
    }
}

pub fn build_pass_context<'a>(rrdevice: &'a RRDevice, data: &'a mut AppData) -> PassContext<'a> {
    PassContext {
        rrdevice,
        graphics: &data.graphics_resources,
        buffers: &data.buffer_registry,
        pipelines: &data.pipeline_storage,
        raytracing: &data.raytracing,
        hdr_buffer: data.viewport.hdr_buffer.as_ref(),
        offscreen: data.viewport.offscreen.as_ref(),
        bloom_chain: data.viewport.bloom_chain.as_ref(),
        dof_buffer: data.viewport.dof_buffer.as_ref(),
        auto_exposure_buffers: data.viewport.auto_exposure_buffers.as_ref(),
        hdr_grid_pipeline_id: data.viewport.hdr_grid_pipeline_id,
        transient: &mut data.viewport.transient,
        frame_transients: &data.frame_transients,
        onion_skin_gpu: data.onion_skin_gpu.as_ref(),
        world: &data.ecs_world,
        assets: &data.ecs_assets,
    }
}

pub struct RenderContext<'a> {
    pub instance: &'a Instance,
    pub device: &'a RRDevice,
    pub command_pool: Rc<RRCommandPool>,
    pub graphics: &'a mut GraphicsResources,
    pub raytracing: &'a mut RayTracingData,
    pub buffer_registry: &'a mut GpuBufferRegistry,
}

impl<'a> RenderContext<'a> {
    pub fn create_backend(&mut self) -> VulkanBackend<'_> {
        VulkanBackend::new(
            self.instance,
            self.device,
            self.command_pool.clone(),
            self.graphics,
            self.raytracing,
            self.buffer_registry,
        )
    }
}
