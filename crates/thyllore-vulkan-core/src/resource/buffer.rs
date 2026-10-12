use crate::command::*;
use crate::core::device::*;
use crate::data::UniformBufferObject;
use crate::resource::allocation::{create_buffer_allocation, BufferAllocation};
use crate::resource::gpu_resource::GpuResource;
use crate::vulkan::*;
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct RRUniformBuffer {
    pub allocation: BufferAllocation,
    #[gpu_resource(skip)]
    pub uniform_buffer_object: UniformBufferObject,
}

impl RRUniformBuffer {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        uniform_buffer_object: UniformBufferObject,
        name: &str,
    ) -> Result<Self> {
        let mut rruniform_buffer = Self::default();
        let allocation = create_buffer_allocation(
            instance,
            rrdevice,
            size_of::<UniformBufferObject>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        log!(
            "RRUniformBuffer::new [{}] buffer={:?}, memory={:?}",
            name,
            allocation.buffer,
            allocation.memory
        );

        rruniform_buffer.allocation = allocation;
        rruniform_buffer.uniform_buffer_object = uniform_buffer_object;
        Ok(rruniform_buffer)
    }

    pub unsafe fn update(
        &mut self,
        rrdevice: &RRDevice,
        ubo: &UniformBufferObject,
    ) -> anyhow::Result<()> {
        let memory = rrdevice.device.map_memory(
            self.allocation.memory,
            0,
            size_of::<UniformBufferObject>() as u64,
            vk::MemoryMapFlags::empty(),
        )?;
        memcpy(ubo, memory.cast(), 1);
        rrdevice.device.unmap_memory(self.allocation.memory);

        self.uniform_buffer_object = ubo.clone();

        Ok(())
    }
}

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct RRIndexBuffer {
    pub allocation: BufferAllocation,
    #[gpu_resource(skip)]
    pub indices: u32,
}

impl RRIndexBuffer {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        rr_command_pool: &RRCommandPool,
        size: u64,
        data: *const c_void,
        length: usize,
    ) -> Result<Self> {
        let mut rrindex_buffer = RRIndexBuffer::default();
        let mut staging_allocation = create_buffer_allocation(
            instance,
            rrdevice,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;
        let map_memory = rrdevice.device.map_memory(
            staging_allocation.memory,
            0,
            size,
            vk::MemoryMapFlags::empty(),
        )?;

        memcpy(data, map_memory.cast(), size as usize);
        rrdevice.device.unmap_memory(staging_allocation.memory);

        let allocation = create_buffer_allocation(
            instance,
            rrdevice,
            size,
            vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        copy_buffer(
            rrdevice,
            rr_command_pool,
            staging_allocation.buffer,
            allocation.buffer,
            size,
        )?;

        GpuResource::destroy_gpu(&mut staging_allocation, rrdevice);

        rrindex_buffer.allocation = allocation;
        rrindex_buffer.indices = length as u32;

        Ok(rrindex_buffer)
    }
}

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct RRVertexBuffer {
    pub allocation: BufferAllocation,
    #[gpu_resource(skip)]
    pub vertices: u32,
    pub staging_allocation: BufferAllocation,
    #[gpu_resource(skip)]
    pub staging_capacity: u64,
}

impl RRVertexBuffer {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        rr_command_pool: &RRCommandPool,
        size: u64,
        data: *const c_void,
        length: usize,
    ) -> Result<Self> {
        let mut rrvertex_buffer = RRVertexBuffer::default();
        let staging_allocation = create_buffer_allocation(
            instance,
            rrdevice,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;
        let map_memory = rrdevice.device.map_memory(
            staging_allocation.memory,
            0,
            size,
            vk::MemoryMapFlags::empty(),
        )?;

        memcpy(data, map_memory.cast(), size as usize);
        rrdevice.device.unmap_memory(staging_allocation.memory);

        let allocation = create_buffer_allocation(
            instance,
            rrdevice,
            size,
            vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        copy_buffer(
            rrdevice,
            rr_command_pool,
            staging_allocation.buffer,
            allocation.buffer,
            size,
        )?;

        rrvertex_buffer.allocation = allocation;
        rrvertex_buffer.vertices = length as u32;
        rrvertex_buffer.staging_allocation = staging_allocation;
        rrvertex_buffer.staging_capacity = size;

        Ok(rrvertex_buffer)
    }

    pub unsafe fn update(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rr_command_pool: &RRCommandPool,
        size: u64,
        data: *const c_void,
        length: usize,
    ) -> Result<()> {
        if self.staging_allocation.is_destroyed() || size > self.staging_capacity {
            self.staging_allocation.destroy_gpu(rrdevice);
            let new_capacity = size * 2;
            self.staging_allocation = create_buffer_allocation(
                instance,
                rrdevice,
                new_capacity,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            )?;
            self.staging_capacity = new_capacity;
        }

        let map_memory = rrdevice.device.map_memory(
            self.staging_allocation.memory,
            0,
            size,
            vk::MemoryMapFlags::empty(),
        )?;

        memcpy(data, map_memory.cast(), size as usize);
        rrdevice.device.unmap_memory(self.staging_allocation.memory);

        copy_buffer(
            rrdevice,
            rr_command_pool,
            self.staging_allocation.buffer,
            self.allocation.buffer,
            size,
        )?;

        self.vertices = length as u32;

        Ok(())
    }
}

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct RRBuffer {
    pub allocation: BufferAllocation,
    #[gpu_resource(skip)]
    pub indices: u32,
}

impl RRBuffer {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<Self> {
        let mut rrbuffer = RRBuffer::default();
        rrbuffer.allocation =
            create_buffer_allocation(instance, rrdevice, size, usage, properties)?;
        Ok(rrbuffer)
    }
}

pub unsafe fn create_buffer(
    instance: &Instance,
    rrdevice: &RRDevice,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let mut allocation = create_buffer_allocation(instance, rrdevice, size, usage, properties)?;
    let buffer = std::mem::take(&mut allocation.buffer);
    let memory = std::mem::take(&mut allocation.memory);
    Ok((buffer, memory))
}

pub unsafe fn copy_buffer(
    rrdevice: &RRDevice,
    rrcommand_pool: &RRCommandPool,
    source: vk::Buffer,
    destination: vk::Buffer,
    size: vk::DeviceSize,
) -> Result<()> {
    let command_buffer = begin_single_time_commands(rrdevice, rrcommand_pool.command_pool)?;
    let regions = vk::BufferCopy::builder().size(size);
    rrdevice
        .device
        .cmd_copy_buffer(command_buffer, source, destination, &[regions]);
    end_single_time_commands(
        rrdevice,
        rrdevice.graphics_queue,
        rrcommand_pool.command_pool,
        command_buffer,
    )?;

    Ok(())
}

pub unsafe fn copy_buffer_to_image(
    rrdevice: &RRDevice,
    rrcommand_buffer: &RRCommandPool,
    buffer: vk::Buffer,
    image: vk::Image,
    width: u32,
    height: u32,
) -> Result<()> {
    let command_buffer = begin_single_time_commands(rrdevice, rrcommand_buffer.command_pool)?;
    let subresources = vk::ImageSubresourceLayers::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .mip_level(0)
        .base_array_layer(0)
        .layer_count(1);

    let region = vk::BufferImageCopy::builder()
        .buffer_offset(0)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(subresources)
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        });

    rrdevice.device.cmd_copy_buffer_to_image(
        command_buffer,
        buffer,
        image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &[region],
    );

    end_single_time_commands(
        rrdevice,
        rrdevice.graphics_queue,
        rrcommand_buffer.command_pool,
        command_buffer,
    )?;

    Ok(())
}
