use crate::resource::GpuResource;
use vulkanalia::prelude::v1_0::*;

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct BufferAllocation {
    pub memory: vk::DeviceMemory,
    pub buffer: vk::Buffer,
}

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct ImageAllocation {
    pub memory: vk::DeviceMemory,
    pub image: vk::Image,
}

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct TextureImage {
    pub allocation: ImageAllocation,
    #[gpu_resource(skip)]
    pub mip_levels: u32,
}
