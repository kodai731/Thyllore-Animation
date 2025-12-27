use crate::vulkanr::command::*;
use crate::vulkanr::core::device::*;
use crate::vulkanr::resource::image::*;
use crate::vulkanr::core::swapchain::*;
use crate::vulkanr::vulkan::*;
#[derive(Clone, Debug, Default)]
pub struct RRGBuffer {
    // World space position (RGB = xyz, A = unused)
    pub position_image: vk::Image,
    pub position_image_memory: vk::DeviceMemory,
    pub position_image_view: vk::ImageView,

    // World space normal (RGB = xyz, A = unused)
    pub normal_image: vk::Image,
    pub normal_image_memory: vk::DeviceMemory,
    pub normal_image_view: vk::ImageView,

    // Albedo / Base Color (RGB = color, A = alpha)
    pub albedo_image: vk::Image,
    pub albedo_image_memory: vk::DeviceMemory,
    pub albedo_image_view: vk::ImageView,

    // Shadow mask (R = shadow factor, 0.0 = shadowed, 1.0 = lit)
    pub shadow_mask_image: vk::Image,
    pub shadow_mask_image_memory: vk::DeviceMemory,
    pub shadow_mask_image_view: vk::ImageView,

    pub width: u32,
    pub height: u32,
}

impl RRGBuffer {
    /// Create G-Buffer images for the given resolution
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        // Create position buffer (RGBA32F for high precision world coordinates)
        let (position_image, position_image_memory) = create_image(
            instance,
            rrdevice,
            width,
            height,
            1, // mip_levels
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

        // Create normal buffer (RGBA32F for normals)
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

        // Create albedo buffer (RGBA8_UNORM for base color/texture)
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

        // Create shadow mask buffer (R32F for shadow factor)
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

    /// Destroy all G-Buffer resources
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

    /// Transition all G-Buffer images to the appropriate layouts
    pub unsafe fn transition_layouts(
        &self,
        rrdevice: &RRDevice,
        command_pool: vk::CommandPool,
    ) -> Result<()> {
        // Transition position and normal to COLOR_ATTACHMENT_OPTIMAL for rendering
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

        // Transition shadow mask to GENERAL for compute shader access
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
