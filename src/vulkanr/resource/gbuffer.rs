use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::resource::image::{create_image, create_image_view, transition_image_layout};

#[derive(Clone, Debug, Default)]
pub struct RRGBuffer {
    pub position_image: vk::Image,
    pub position_image_memory: vk::DeviceMemory,
    pub position_image_view: vk::ImageView,

    pub normal_image: vk::Image,
    pub normal_image_memory: vk::DeviceMemory,
    pub normal_image_view: vk::ImageView,

    pub albedo_image: vk::Image,
    pub albedo_image_memory: vk::DeviceMemory,
    pub albedo_image_view: vk::ImageView,

    pub shadow_mask_image: vk::Image,
    pub shadow_mask_image_memory: vk::DeviceMemory,
    pub shadow_mask_image_view: vk::ImageView,

    pub width: u32,
    pub height: u32,
}

impl RRGBuffer {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let (position_image, position_image_memory) = create_image(
            instance,
            rrdevice,
            width,
            height,
            1,
            vk::SampleCountFlags::_1,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let position_image_view = create_image_view(
            rrdevice,
            position_image,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let (normal_image, normal_image_memory) = create_image(
            instance,
            rrdevice,
            width,
            height,
            1,
            vk::SampleCountFlags::_1,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let normal_image_view = create_image_view(
            rrdevice,
            normal_image,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let (albedo_image, albedo_image_memory) = create_image(
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
            albedo_image,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let (shadow_mask_image, shadow_mask_image_memory) = create_image(
            instance,
            rrdevice,
            width,
            height,
            1,
            vk::SampleCountFlags::_1,
            vk::Format::R32_SFLOAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let shadow_mask_image_view = create_image_view(
            rrdevice,
            shadow_mask_image,
            vk::Format::R32_SFLOAT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        log::info!(
            "Created G-Buffer: {}x{} (position, normal, albedo, shadow mask)",
            width,
            height
        );

        Ok(Self {
            position_image,
            position_image_memory,
            position_image_view,
            normal_image,
            normal_image_memory,
            normal_image_view,
            albedo_image,
            albedo_image_memory,
            albedo_image_view,
            shadow_mask_image,
            shadow_mask_image_memory,
            shadow_mask_image_view,
            width,
            height,
        })
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        device.destroy_image_view(self.position_image_view, None);
        device.destroy_image(self.position_image, None);
        device.free_memory(self.position_image_memory, None);

        device.destroy_image_view(self.normal_image_view, None);
        device.destroy_image(self.normal_image, None);
        device.free_memory(self.normal_image_memory, None);

        device.destroy_image_view(self.albedo_image_view, None);
        device.destroy_image(self.albedo_image, None);
        device.free_memory(self.albedo_image_memory, None);

        device.destroy_image_view(self.shadow_mask_image_view, None);
        device.destroy_image(self.shadow_mask_image, None);
        device.free_memory(self.shadow_mask_image_memory, None);

        log::info!("Destroyed G-Buffer");
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
            self.position_image,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            1,
        )?;

        transition_image_layout(
            rrdevice,
            rrdevice.graphics_queue,
            command_pool,
            self.normal_image,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            1,
        )?;

        transition_image_layout(
            rrdevice,
            rrdevice.graphics_queue,
            command_pool,
            self.albedo_image,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            1,
        )?;

        transition_image_layout(
            rrdevice,
            rrdevice.graphics_queue,
            command_pool,
            self.shadow_mask_image,
            vk::Format::R32_SFLOAT,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
            1,
        )?;

        Ok(())
    }
}
