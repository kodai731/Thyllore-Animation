use std::marker::PhantomData;

use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::DeviceV1_2;

use crate::core::device::RRDevice;
use crate::resource::allocation::{create_buffer_allocation, BufferAllocation};
use crate::resource::gpu_resource::GpuResource;
use crate::vulkan::Instance;
use thyllore_spirv_reflect::GpuBlock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placement {
    #[default]
    HostMapped,
    DeviceUpdated,
}

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct UniformBuffer<T: GpuBlock> {
    allocation: BufferAllocation,
    #[gpu_resource(skip)]
    slot_stride: vk::DeviceSize,
    #[gpu_resource(skip)]
    slot_count: usize,
    #[gpu_resource(skip)]
    placement: Placement,
    #[gpu_resource(skip)]
    _block: PhantomData<T>,
}

impl<T: GpuBlock> UniformBuffer<T> {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        slot_count: usize,
        placement: Placement,
    ) -> Result<Self> {
        if slot_count == 0 {
            return Err(anyhow!(
                "{} uniform buffer needs at least one slot",
                T::NAME
            ));
        }
        let alignment = rrdevice.min_uniform_buffer_offset_alignment.max(1);
        let slot_stride = (T::SIZE as vk::DeviceSize).div_ceil(alignment) * alignment;

        let usage = match placement {
            Placement::HostMapped => {
                vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            }
            Placement::DeviceUpdated => {
                vk::BufferUsageFlags::UNIFORM_BUFFER
                    | vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            }
        };
        let allocation = create_buffer_allocation(
            instance,
            rrdevice,
            slot_stride * slot_count as vk::DeviceSize,
            usage,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        Ok(Self {
            allocation,
            slot_stride,
            slot_count,
            placement,
            _block: PhantomData,
        })
    }

    pub fn handle(&self) -> vk::Buffer {
        self.allocation.buffer
    }

    pub fn block_size(&self) -> vk::DeviceSize {
        T::SIZE as vk::DeviceSize
    }

    pub fn slot_offset(&self, slot: usize) -> Result<vk::DeviceSize> {
        if slot >= self.slot_count {
            return Err(anyhow!(
                "{} slot {slot} is out of range (slot count {})",
                T::NAME,
                self.slot_count
            ));
        }
        Ok(self.slot_stride * slot as vk::DeviceSize)
    }

    /// Device address of one slot, for shaders that reach the block through a buffer reference.
    pub unsafe fn slot_address(
        &self,
        device: &vulkanalia::Device,
        slot: usize,
    ) -> Result<vk::DeviceAddress> {
        let base = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(self.allocation.buffer),
        );
        Ok(base + self.slot_offset(slot)?)
    }

    pub unsafe fn write_slot(&self, rrdevice: &RRDevice, slot: usize, value: &T) -> Result<()> {
        let offset = self.slot_offset(slot)?;
        let bytes = value.as_bytes();
        let mapped = rrdevice.device.map_memory(
            self.allocation.memory,
            offset,
            bytes.len() as vk::DeviceSize,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), mapped.cast::<u8>(), bytes.len());
        rrdevice.device.unmap_memory(self.allocation.memory);
        Ok(())
    }

    /// Transfers inside the command buffer so frames in flight never race on one slot.
    pub unsafe fn record_update(
        &self,
        device: &vulkanalia::Device,
        cmd: vk::CommandBuffer,
        slot: usize,
        value: &T,
        reader_stage: vk::PipelineStageFlags,
    ) -> Result<()> {
        if self.placement != Placement::DeviceUpdated {
            return Err(anyhow!(
                "{} uniform buffer is {:?}; record_update needs Placement::DeviceUpdated",
                T::NAME,
                self.placement
            ));
        }
        let offset = self.slot_offset(slot)?;

        self.record_barrier(
            device,
            cmd,
            offset,
            (reader_stage, vk::AccessFlags::UNIFORM_READ),
            (
                vk::PipelineStageFlags::TRANSFER,
                vk::AccessFlags::TRANSFER_WRITE,
            ),
        );
        device.cmd_update_buffer(cmd, self.allocation.buffer, offset, value.as_bytes());
        self.record_barrier(
            device,
            cmd,
            offset,
            (
                vk::PipelineStageFlags::TRANSFER,
                vk::AccessFlags::TRANSFER_WRITE,
            ),
            (reader_stage, vk::AccessFlags::UNIFORM_READ),
        );
        Ok(())
    }

    unsafe fn record_barrier(
        &self,
        device: &vulkanalia::Device,
        cmd: vk::CommandBuffer,
        offset: vk::DeviceSize,
        (src_stage, src_access): (vk::PipelineStageFlags, vk::AccessFlags),
        (dst_stage, dst_access): (vk::PipelineStageFlags, vk::AccessFlags),
    ) {
        let barrier = vk::BufferMemoryBarrier::builder()
            .src_access_mask(src_access)
            .dst_access_mask(dst_access)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .buffer(self.allocation.buffer)
            .offset(offset)
            .size(self.block_size())
            .build();
        device.cmd_pipeline_barrier(
            cmd,
            src_stage,
            dst_stage,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[barrier],
            &[] as &[vk::ImageMemoryBarrier],
        );
    }
}
