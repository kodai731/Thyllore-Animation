use crate::app::{App, AppData};

use crate::vulkanr::command::*;
use crate::vulkanr::context::RenderConfig;
use crate::vulkanr::descriptor::*;
use crate::vulkanr::device::*;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::render::*;
use crate::vulkanr::vulkan::*;

use vulkanalia::Device as VkDevice;

use anyhow::Result;
use std::ptr::copy_nonoverlapping as memcpy;

impl App {
    pub unsafe fn init_imgui_rendering(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        imgui: &mut imgui::Context,
        rrcommand_pool: &RRCommandPool,
        rrrender: &RRRender,
    ) -> Result<()> {
        log!("Initializing ImGui Vulkan rendering resources");

        let font_atlas = imgui.fonts();
        let font_texture = font_atlas.build_rgba32_texture();
        let width = font_texture.width;
        let height = font_texture.height;
        let font_data: &[u8] = &font_texture.data;
        log!("Font texture size: {}x{}", width, height);

        let (image, image_memory) = Self::create_font_image(instance, rrdevice, width, height)?;

        Self::upload_font_data_via_staging(
            instance,
            rrdevice,
            rrcommand_pool,
            image,
            font_data,
            width,
            height,
        )?;

        let image_view = Self::create_font_image_view(&rrdevice.device, image)?;
        let sampler = Self::create_font_sampler(&rrdevice.device)?;

        let (descriptor_set_layout, descriptor_set) =
            Self::setup_imgui_descriptors(rrdevice, image_view, sampler)?;

        let msaa_samples = {
            let render_config = data.ecs_world.resource::<RenderConfig>();
            if !render_config.msaa_samples.is_empty() {
                render_config.msaa_samples
            } else {
                vk::SampleCountFlags::_8
            }
        };

        let imgui_pipeline = RRPipeline::new_imgui(
            rrdevice,
            rrrender,
            &descriptor_set_layout,
            &IMGUI,
            msaa_samples,
        )?;

        data.imgui.pipeline = Some(imgui_pipeline.pipeline);
        data.imgui.pipeline_layout = Some(imgui_pipeline.pipeline_layout);
        data.imgui.descriptor_set = Some(descriptor_set);
        data.imgui.descriptor_set_layout = Some(descriptor_set_layout);
        data.imgui.font_image = Some(image);
        data.imgui.font_image_memory = Some(image_memory);
        data.imgui.font_image_view = Some(image_view);
        data.imgui.sampler = Some(sampler);

        log!("ImGui rendering resources initialized successfully");
        log!("  Pipeline: {:?}", imgui_pipeline.pipeline);
        log!("  Descriptor Set: {:?}", descriptor_set);

        Ok(())
    }

    unsafe fn create_font_image(
        instance: &Instance,
        rrdevice: &RRDevice,
        width: u32,
        height: u32,
    ) -> Result<(vk::Image, vk::DeviceMemory)> {
        let extent = vk::Extent3D {
            width,
            height,
            depth: 1,
        };

        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = rrdevice.device.create_image(&image_info, None)?;

        let requirements = rrdevice.device.get_image_memory_requirements(image);
        let memory_type_index = get_memory_type_index(
            instance,
            rrdevice.physical_device,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            requirements,
        )?;

        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index);

        let image_memory = rrdevice.device.allocate_memory(&allocate_info, None)?;
        rrdevice.device.bind_image_memory(image, image_memory, 0)?;

        Ok((image, image_memory))
    }

    unsafe fn upload_font_data_via_staging(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
        image: vk::Image,
        font_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<()> {
        let buffer_size = (width * height * 4) as vk::DeviceSize;

        let buffer_info = vk::BufferCreateInfo::builder()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let staging_buffer = rrdevice.device.create_buffer(&buffer_info, None)?;
        let buffer_requirements = rrdevice
            .device
            .get_buffer_memory_requirements(staging_buffer);

        let memory_type_index = get_memory_type_index(
            instance,
            rrdevice.physical_device,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            buffer_requirements,
        )?;

        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(buffer_requirements.size)
            .memory_type_index(memory_type_index);

        let staging_buffer_memory = rrdevice.device.allocate_memory(&allocate_info, None)?;
        rrdevice
            .device
            .bind_buffer_memory(staging_buffer, staging_buffer_memory, 0)?;

        let memory_ptr = rrdevice.device.map_memory(
            staging_buffer_memory,
            0,
            buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        memcpy(font_data.as_ptr(), memory_ptr.cast(), font_data.len());
        rrdevice.device.unmap_memory(staging_buffer_memory);

        Self::transition_image_layout_and_copy(
            &rrdevice.device,
            rrcommand_pool,
            &rrdevice.graphics_queue,
            image,
            staging_buffer,
            width,
            height,
        )?;

        rrdevice.device.destroy_buffer(staging_buffer, None);
        rrdevice.device.free_memory(staging_buffer_memory, None);

        Ok(())
    }

    unsafe fn create_font_image_view(device: &VkDevice, image: vk::Image) -> Result<vk::ImageView> {
        let view_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        Ok(device.create_image_view(&view_info, None)?)
    }

    unsafe fn create_font_sampler(device: &VkDevice) -> Result<vk::Sampler> {
        let sampler_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .min_lod(0.0)
            .max_lod(1.0);

        Ok(device.create_sampler(&sampler_info, None)?)
    }

    unsafe fn setup_imgui_descriptors(
        rrdevice: &RRDevice,
        image_view: vk::ImageView,
        sampler: vk::Sampler,
    ) -> Result<(ReflectedSetLayout, vk::DescriptorSet)> {
        let layout = ReflectedSetLayout::create(rrdevice, &imgui_layout_spec())?;
        let descriptor_set = layout.allocate_set(rrdevice)?;

        layout
            .writer(descriptor_set)
            .image(
                shader_bindings::imgui::TEX_SAMPLER,
                image_view,
                sampler,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            )?
            .apply(rrdevice);

        Ok((layout, descriptor_set))
    }
}
