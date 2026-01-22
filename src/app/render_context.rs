use std::rc::Rc;

use crate::app::graphics_resource::GraphicsResources;
use crate::app::raytracing::RayTracingData;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::core::device::RRDevice;
use crate::vulkanr::resource::GpuBufferRegistry;
use crate::vulkanr::vulkan::Instance;
use crate::vulkanr::VulkanBackend;

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
