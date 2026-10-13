use anyhow::Result;
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::prelude::v1_0::*;

use crate::core::device::RRDevice;
use crate::resource::allocation::{create_buffer, BufferAllocation};
use crate::resource::gpu_resource::GpuResource;

pub struct DynamicUniformBuffer<T: Copy> {
    allocation: BufferAllocation,
    mapped_ptr: *mut c_void,
    alignment: u64,
    element_aligned_size: u64,
    capacity: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Copy> DynamicUniformBuffer<T> {
    pub unsafe fn new(instance: &Instance, rrdevice: &RRDevice, capacity: usize) -> Result<Self> {
        let min_alignment = rrdevice.min_uniform_buffer_offset_alignment;
        let element_size = size_of::<T>() as u64;
        let element_aligned_size = Self::align_size(element_size, min_alignment);
        let total_size = element_aligned_size * capacity as u64;

        let allocation = create_buffer(
            instance,
            rrdevice,
            total_size,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let mapped_ptr = rrdevice.device.map_memory(
            allocation.memory,
            0,
            total_size,
            vk::MemoryMapFlags::empty(),
        )?;

        Ok(Self {
            allocation,
            mapped_ptr,
            alignment: min_alignment,
            element_aligned_size,
            capacity,
            _marker: std::marker::PhantomData,
        })
    }

    fn align_size(size: u64, alignment: u64) -> u64 {
        (size + alignment - 1) & !(alignment - 1)
    }

    pub fn buffer(&self) -> vk::Buffer {
        self.allocation.buffer
    }

    pub fn alignment(&self) -> u64 {
        self.alignment
    }

    pub fn element_aligned_size(&self) -> u64 {
        self.element_aligned_size
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn dynamic_offset(&self, index: usize) -> u32 {
        (self.element_aligned_size * index as u64) as u32
    }

    pub unsafe fn update(&self, index: usize, data: &T) -> Result<()> {
        if index >= self.capacity {
            return Err(anyhow::anyhow!(
                "Index {} out of bounds (capacity: {})",
                index,
                self.capacity
            ));
        }

        let offset = self.element_aligned_size * index as u64;
        let dst = (self.mapped_ptr as *mut u8).add(offset as usize) as *mut T;
        memcpy(data, dst, 1);

        Ok(())
    }

    pub unsafe fn update_range(&self, start_index: usize, data: &[T]) -> Result<()> {
        if start_index + data.len() > self.capacity {
            return Err(anyhow::anyhow!(
                "Range {}..{} out of bounds (capacity: {})",
                start_index,
                start_index + data.len(),
                self.capacity
            ));
        }

        for (i, item) in data.iter().enumerate() {
            self.update(start_index + i, item)?;
        }

        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        if self.mapped_ptr != std::ptr::null_mut() {
            rrdevice.device.unmap_memory(self.allocation.memory);
            self.mapped_ptr = std::ptr::null_mut();
        }

        self.allocation.destroy_gpu(rrdevice);
    }
}

impl<T: Copy> Drop for DynamicUniformBuffer<T> {
    fn drop(&mut self) {
        if self.allocation.buffer != vk::Buffer::null() {
            log_warn!("DynamicUniformBuffer dropped without calling destroy()");
        }
    }
}
