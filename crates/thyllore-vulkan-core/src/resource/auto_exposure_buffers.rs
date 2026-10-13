use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::resource::allocation::{create_buffer, BufferAllocation};
use crate::resource::GpuResource;
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

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct AutoExposureBuffers {
    pub readback_allocations: [BufferAllocation; 2],
    pub luminance_allocation: BufferAllocation,
    pub histogram_allocation: BufferAllocation,
    #[gpu_resource(skip)]
    pub width: u32,
    #[gpu_resource(skip)]
    pub height: u32,
}

impl AutoExposureBuffers {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let histogram_allocation = create_buffer(
            instance,
            rrdevice,
            HISTOGRAM_BUFFER_SIZE,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let luminance_allocation = create_buffer(
            instance,
            rrdevice,
            LUMINANCE_BUFFER_SIZE,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        Self::zero_luminance_buffer(&rrdevice.device, luminance_allocation.memory)?;

        let mut readback_allocations: [BufferAllocation; 2] = Default::default();

        for readback_allocation in &mut readback_allocations {
            let allocation = create_buffer(
                instance,
                rrdevice,
                LUMINANCE_BUFFER_SIZE,
                vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            Self::zero_readback_buffer(&rrdevice.device, allocation.memory)?;
            *readback_allocation = allocation;
        }

        log!("Created AutoExposure buffers: {}x{}", width, height);

        Ok(Self {
            histogram_allocation,
            luminance_allocation,
            readback_allocations,
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

    pub unsafe fn read_luminance(
        &self,
        device: &vulkanalia::Device,
        slot: usize,
    ) -> Option<AutoExposureReadback> {
        let data = device
            .map_memory(
                self.readback_allocations[slot].memory,
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

        device.unmap_memory(self.readback_allocations[slot].memory);
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

        self.destroy_gpu(rrdevice);
        let new_buf = Self::new(instance, rrdevice, new_width, new_height)?;
        *self = new_buf;

        Ok(())
    }
}
