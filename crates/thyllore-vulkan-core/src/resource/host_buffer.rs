use crate::core::device::RRDevice;
use crate::resource::buffer::create_buffer;
use crate::types::Instance;
use crate::vulkan::*;

#[derive(Debug, Clone, Copy)]
pub enum HostBufferUsage {
    Vertex,
    Index,
}

impl HostBufferUsage {
    fn to_flags(self) -> vk::BufferUsageFlags {
        match self {
            HostBufferUsage::Vertex => vk::BufferUsageFlags::VERTEX_BUFFER,
            HostBufferUsage::Index => vk::BufferUsageFlags::INDEX_BUFFER,
        }
    }
}

#[derive(Debug, Default)]
pub struct HostBuffer {
    pub(crate) buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    pub(crate) size: vk::DeviceSize,
}

impl HostBuffer {
    #[inline]
    pub fn buffer(&self) -> crate::types::Buffer {
        crate::types::Buffer(self.buffer)
    }

    #[inline]
    pub fn size(&self) -> crate::types::DeviceSize {
        self.size
    }

    pub unsafe fn create(
        instance: &Instance,
        rrdevice: &RRDevice,
        size: crate::types::DeviceSize,
        usage: HostBufferUsage,
    ) -> Result<Self> {
        let (buffer, memory) = create_buffer(
            instance,
            rrdevice,
            size,
            usage.to_flags(),
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        Ok(Self {
            buffer,
            memory,
            size,
        })
    }

    pub unsafe fn write_with(
        &self,
        rrdevice: &RRDevice,
        size: crate::types::DeviceSize,
        write: impl FnOnce(&mut [u8]),
    ) -> Result<()> {
        let ptr =
            rrdevice
                .device()
                .map_memory(self.memory, 0, size, vk::MemoryMapFlags::empty())?;

        let slice = std::slice::from_raw_parts_mut(ptr as *mut u8, size as usize);
        write(slice);

        rrdevice.device().unmap_memory(self.memory);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        if self.buffer != vk::Buffer::null() {
            rrdevice.device().destroy_buffer(self.buffer, None);
            self.buffer = vk::Buffer::null();
        }
        if self.memory != vk::DeviceMemory::null() {
            rrdevice.device().free_memory(self.memory, None);
            self.memory = vk::DeviceMemory::null();
        }
        self.size = 0;
    }
}
