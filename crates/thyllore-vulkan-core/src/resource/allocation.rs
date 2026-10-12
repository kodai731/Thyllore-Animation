use crate::core::device::RRDevice;
use crate::resource::GpuResource;
use crate::vulkan::*;

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct BufferAllocation {
    pub memory: vk::DeviceMemory,
    pub buffer: vk::Buffer,
}

pub unsafe fn create_buffer(
    instance: &Instance,
    rrdevice: &RRDevice,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<BufferAllocation> {
    let buffer_info = vk::BufferCreateInfo::builder()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = rrdevice.device.create_buffer(&buffer_info, None)?;
    let requirements = rrdevice.device.get_buffer_memory_requirements(buffer);

    let mut memory_allocate_flags_info =
        vk::MemoryAllocateFlagsInfo::builder().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);

    let mut memory_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(get_memory_type_index(
            instance,
            rrdevice.physical_device,
            properties,
            requirements,
        )?);

    if usage.contains(vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS) {
        memory_info = memory_info.push_next(&mut memory_allocate_flags_info);
    }

    let memory = rrdevice.device.allocate_memory(&memory_info, None)?;
    rrdevice.device.bind_buffer_memory(buffer, memory, 0)?;

    Ok(BufferAllocation { memory, buffer })
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
