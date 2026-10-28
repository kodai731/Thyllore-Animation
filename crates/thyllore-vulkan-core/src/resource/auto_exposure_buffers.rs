use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::resource::buffer::create_buffer;
use crate::resource::gpu_resource::GpuResource;
use crate::types::Instance;
use crate::vulkan::*;

const HISTOGRAM_BIN_COUNT: u32 = 256;
const HISTOGRAM_BUFFER_SIZE: u64 = (HISTOGRAM_BIN_COUNT * std::mem::size_of::<u32>() as u32) as u64;
pub const LUMINANCE_BUFFER_SIZE: u64 = (2 * std::mem::size_of::<f32>() as u32) as u64;

/// The two floats the average pass writes: metered luminance and the adapted exposure multiplier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoExposureReadback {
    pub average_luminance: f32,
    pub adapted_exposure: f32,
}

#[derive(Clone, Debug, Default)]
pub struct AutoExposureBuffers {
    pub(crate) histogram_buffer: vk::Buffer,
    pub(crate) histogram_buffer_memory: vk::DeviceMemory,
    pub(crate) luminance_buffer: vk::Buffer,
    pub(crate) luminance_buffer_memory: vk::DeviceMemory,
    pub(crate) readback_buffers: [vk::Buffer; 2],
    pub(crate) readback_memories: [vk::DeviceMemory; 2],
    pub width: u32,
    pub height: u32,
}

impl AutoExposureBuffers {
    #[inline]
    pub fn histogram_buffer(&self) -> crate::types::Buffer {
        crate::types::Buffer(self.histogram_buffer)
    }

    #[inline]
    pub fn luminance_buffer(&self) -> crate::types::Buffer {
        crate::types::Buffer(self.luminance_buffer)
    }

    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let (histogram_buffer, histogram_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            HISTOGRAM_BUFFER_SIZE,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let (luminance_buffer, luminance_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            LUMINANCE_BUFFER_SIZE,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        Self::zero_luminance_buffer(&rrdevice.device(), luminance_buffer_memory)?;

        let mut readback_buffers = [vk::Buffer::null(); 2];
        let mut readback_memories = [vk::DeviceMemory::null(); 2];

        for i in 0..2 {
            let (buf, mem) = create_buffer(
                instance,
                rrdevice,
                LUMINANCE_BUFFER_SIZE,
                vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            Self::zero_readback_buffer(&rrdevice.device(), mem)?;
            readback_buffers[i] = buf;
            readback_memories[i] = mem;
        }

        log!("Created AutoExposure buffers: {}x{}", width, height);

        Ok(Self {
            histogram_buffer,
            histogram_buffer_memory,
            luminance_buffer,
            luminance_buffer_memory,
            readback_buffers,
            readback_memories,
            width,
            height,
        })
    }

    unsafe fn zero_luminance_buffer(
        device: &vulkanalia::Device,
        memory: vk::DeviceMemory,
    ) -> Result<()> {
        let data = device.map_memory(
            memory,
            0,
            LUMINANCE_BUFFER_SIZE,
            vk::MemoryMapFlags::empty(),
        )?;

        std::ptr::write_bytes(data as *mut u8, 0, LUMINANCE_BUFFER_SIZE as usize);

        device.unmap_memory(memory);
        Ok(())
    }

    unsafe fn zero_readback_buffer(
        device: &vulkanalia::Device,
        memory: vk::DeviceMemory,
    ) -> Result<()> {
        let data = device.map_memory(
            memory,
            0,
            LUMINANCE_BUFFER_SIZE,
            vk::MemoryMapFlags::empty(),
        )?;

        std::ptr::write_bytes(data as *mut u8, 0, LUMINANCE_BUFFER_SIZE as usize);

        device.unmap_memory(memory);
        Ok(())
    }

    pub unsafe fn record_luminance_readback(
        &self,
        rrdevice: &RRDevice,
        command_buffer: crate::types::CommandBuffer,
        slot: usize,
    ) {
        let command_buffer = command_buffer.raw();
        let barrier = vk::BufferMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .buffer(self.luminance_buffer)
            .offset(0)
            .size(LUMINANCE_BUFFER_SIZE)
            .build();

        rrdevice.device().cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[barrier],
            &[] as &[vk::ImageMemoryBarrier],
        );

        rrdevice.device().cmd_copy_buffer(
            command_buffer,
            self.luminance_buffer,
            self.readback_buffers[slot],
            &[vk::BufferCopy::builder()
                .src_offset(0)
                .dst_offset(0)
                .size(LUMINANCE_BUFFER_SIZE)
                .build()],
        );
    }

    pub unsafe fn read_luminance(
        &self,
        rrdevice: &RRDevice,
        slot: usize,
    ) -> Option<AutoExposureReadback> {
        let device = &rrdevice.device();
        let data = device
            .map_memory(
                self.readback_memories[slot],
                0,
                LUMINANCE_BUFFER_SIZE,
                vk::MemoryMapFlags::empty(),
            )
            .ok()?;

        let values = data as *const f32;
        let readback = AutoExposureReadback {
            average_luminance: *values,
            adapted_exposure: *values.add(1),
        };

        device.unmap_memory(self.readback_memories[slot]);
        Some(readback)
    }

    pub unsafe fn resize(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        new_width: u32,
        new_height: u32,
    ) -> Result<()> {
        if new_width == self.width && new_height == self.height {
            return Ok(());
        }

        self.destroy(&rrdevice.device());
        let new_buf = Self::new(instance, rrdevice, new_width, new_height)?;
        *self = new_buf;

        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        if self.histogram_buffer != vk::Buffer::null() {
            device.destroy_buffer(self.histogram_buffer, None);
            self.histogram_buffer = vk::Buffer::null();
        }
        if self.histogram_buffer_memory != vk::DeviceMemory::null() {
            device.free_memory(self.histogram_buffer_memory, None);
            self.histogram_buffer_memory = vk::DeviceMemory::null();
        }

        if self.luminance_buffer != vk::Buffer::null() {
            device.destroy_buffer(self.luminance_buffer, None);
            self.luminance_buffer = vk::Buffer::null();
        }
        if self.luminance_buffer_memory != vk::DeviceMemory::null() {
            device.free_memory(self.luminance_buffer_memory, None);
            self.luminance_buffer_memory = vk::DeviceMemory::null();
        }

        for i in 0..2 {
            if self.readback_buffers[i] != vk::Buffer::null() {
                device.destroy_buffer(self.readback_buffers[i], None);
                self.readback_buffers[i] = vk::Buffer::null();
            }
            if self.readback_memories[i] != vk::DeviceMemory::null() {
                device.free_memory(self.readback_memories[i], None);
                self.readback_memories[i] = vk::DeviceMemory::null();
            }
        }

        log!("Destroyed AutoExposure buffers");
    }
}

impl Drop for AutoExposureBuffers {
    fn drop(&mut self) {
        if self.histogram_buffer != vk::Buffer::null() {
            log_warn!("AutoExposureBuffers dropped without calling destroy()");
        }
    }
}

impl GpuResource for AutoExposureBuffers {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device());
    }

    fn is_destroyed(&self) -> bool {
        self.histogram_buffer.is_destroyed()
    }
}
