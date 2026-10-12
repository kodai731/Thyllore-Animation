use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::resource::allocation::{create_buffer, BufferAllocation, ImageAllocation};
use crate::resource::gpu_resource::GpuResource;
use crate::resource::image::{create_image, create_image_view, transition_image_layout};

/// Object id at offset 0, world position at `READBACK_POSITION_OFFSET`. Picking reads both from
/// the same pixel so a click can compare what an effect covers against what the surface covers.
pub const READBACK_POSITION_OFFSET: vk::DeviceSize = 16;
pub const READBACK_STAGING_SIZE: vk::DeviceSize = READBACK_POSITION_OFFSET + 16;

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct RRGBuffer {
    pub position: ImageAllocation,
    pub normal: ImageAllocation,
    pub albedo: ImageAllocation,
    pub object_id: ImageAllocation,
    pub shadow_mask: ImageAllocation,

    pub position_image_view: vk::ImageView,
    pub normal_image_view: vk::ImageView,
    pub albedo_image_view: vk::ImageView,
    pub object_id_image_view: vk::ImageView,
    pub shadow_mask_image_view: vk::ImageView,

    pub readback_staging: BufferAllocation,

    #[gpu_resource(skip)]
    pub width: u32,
    #[gpu_resource(skip)]
    pub height: u32,
}

impl RRGBuffer {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let position = create_image(
            instance,
            rrdevice,
            width,
            height,
            1,
            vk::SampleCountFlags::_1,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let position_image_view = create_image_view(
            rrdevice,
            position.image,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let normal = create_image(
            instance,
            rrdevice,
            width,
            height,
            1,
            vk::SampleCountFlags::_1,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let normal_image_view = create_image_view(
            rrdevice,
            normal.image,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let albedo = create_image(
            instance,
            rrdevice,
            width,
            height,
            1,
            vk::SampleCountFlags::_1,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let albedo_image_view = create_image_view(
            rrdevice,
            albedo.image,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let object_id = create_image(
            instance,
            rrdevice,
            width,
            height,
            1,
            vk::SampleCountFlags::_1,
            vk::Format::R32_UINT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let object_id_image_view = create_image_view(
            rrdevice,
            object_id.image,
            vk::Format::R32_UINT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let shadow_mask = create_image(
            instance,
            rrdevice,
            width,
            height,
            1,
            vk::SampleCountFlags::_1,
            vk::Format::R32_SFLOAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let shadow_mask_image_view = create_image_view(
            rrdevice,
            shadow_mask.image,
            vk::Format::R32_SFLOAT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let readback_staging = create_buffer(
            instance,
            rrdevice,
            READBACK_STAGING_SIZE,
            vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        log!(
            "Created G-Buffer: {}x{} (position, normal, albedo, object_id, shadow mask)",
            width,
            height
        );

        Ok(Self {
            position,
            normal,
            albedo,
            object_id,
            shadow_mask,
            position_image_view,
            normal_image_view,
            albedo_image_view,
            object_id_image_view,
            shadow_mask_image_view,
            readback_staging,
            width,
            height,
        })
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
        if new_width == 0 || new_height == 0 {
            return Ok(());
        }

        self.destroy_gpu(rrdevice);
        *self = RRGBuffer::new(instance, rrdevice, new_width, new_height)?;
        Ok(())
    }

    pub unsafe fn transition_layouts(
        &self,
        rrdevice: &RRDevice,
        command_pool: vk::CommandPool,
    ) -> Result<()> {
        transition_image_layout(
            rrdevice,
            rrdevice.graphics_queue,
            command_pool,
            self.position.image,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
            1,
        )?;

        transition_image_layout(
            rrdevice,
            rrdevice.graphics_queue,
            command_pool,
            self.normal.image,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
            1,
        )?;

        transition_image_layout(
            rrdevice,
            rrdevice.graphics_queue,
            command_pool,
            self.albedo.image,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            1,
        )?;

        transition_image_layout(
            rrdevice,
            rrdevice.graphics_queue,
            command_pool,
            self.object_id.image,
            vk::Format::R32_UINT,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            1,
        )?;

        transition_image_layout(
            rrdevice,
            rrdevice.graphics_queue,
            command_pool,
            self.shadow_mask.image,
            vk::Format::R32_SFLOAT,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
            1,
        )?;

        Ok(())
    }
}
