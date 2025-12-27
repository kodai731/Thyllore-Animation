use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
use rust_rendering::vulkanr::pipeline::RRPipeline;
use rust_rendering::vulkanr::descriptor::RRDescriptorSet;
use rust_rendering::vulkanr::core::{Device, RRDevice};
use rust_rendering::vulkanr::resource::image::{create_image, create_image_view, transition_image_layout};
use rust_rendering::vulkanr::render::RRRender;
use rust_rendering::vulkanr::render::pass::get_depth_format;

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

pub unsafe fn create_gbuffer_framebuffer(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &mut RRRender,
    gbuffer: &RRGBuffer,
) -> Result<()> {
    let (depth_image, depth_image_memory) = create_image(
        instance,
        rrdevice,
        gbuffer.width,
        gbuffer.height,
        1,
        vk::SampleCountFlags::_1,
        get_depth_format(instance, rrdevice)?,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    let depth_image_view = create_image_view(
        rrdevice,
        depth_image,
        get_depth_format(instance, rrdevice)?,
        vk::ImageAspectFlags::DEPTH,
        1,
    )?;

    rrrender.gbuffer_depth_image = depth_image;
    rrrender.gbuffer_depth_image_memory = depth_image_memory;
    rrrender.gbuffer_depth_image_view = depth_image_view;

    let attachments = [
        gbuffer.position_image_view,
        gbuffer.normal_image_view,
        gbuffer.albedo_image_view,
        depth_image_view,
    ];

    let info = vk::FramebufferCreateInfo::builder()
        .render_pass(rrrender.gbuffer_render_pass)
        .attachments(&attachments)
        .width(gbuffer.width)
        .height(gbuffer.height)
        .layers(1);

    rrrender.gbuffer_framebuffer = rrdevice.device.create_framebuffer(&info, None)?;

    log::info!("Created G-Buffer framebuffer: {}x{}", gbuffer.width, gbuffer.height);
    Ok(())
}

pub struct GBufferPass<'a> {
    gbuffer: &'a RRGBuffer,
    pipeline: &'a RRPipeline,
    descriptor_set: &'a RRDescriptorSet,
    device: &'a Device,
}

impl<'a> GBufferPass<'a> {
    pub fn new(app: &'a App) -> Result<Self> {
        let gbuffer = app.data.gbuffer.as_ref()
            .ok_or_else(|| anyhow!("G-Buffer not initialized"))?;
        let pipeline = app.data.gbuffer_pipeline.as_ref()
            .ok_or_else(|| anyhow!("G-Buffer pipeline not initialized"))?;
        let descriptor_set = app.data.gbuffer_descriptor_set.as_ref()
            .ok_or_else(|| anyhow!("G-Buffer descriptor set not initialized"))?;

        Ok(Self {
            gbuffer,
            pipeline,
            descriptor_set,
            device: &app.rrdevice.device,
        })
    }

    pub unsafe fn record(
        &self,
        command_buffer: vk::CommandBuffer,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        image_index: usize,
    ) -> Result<()> {
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(vk::Extent2D {
                width: self.gbuffer.width,
                height: self.gbuffer.height,
            });

        let clear_values = self.create_clear_values();

        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass)
            .framebuffer(framebuffer)
            .render_area(render_area)
            .clear_values(&clear_values);

        self.device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_info,
            vk::SubpassContents::INLINE,
        );

        self.bind_pipeline_and_state(command_buffer);
        self.draw_meshes(command_buffer, image_index)?;

        self.device.cmd_end_render_pass(command_buffer);

        Ok(())
    }

    fn create_clear_values(&self) -> [vk::ClearValue; 4] {
        let position_clear = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        };
        let normal_clear = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        };
        let albedo_clear = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        };
        let depth_clear = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        };

        [position_clear, normal_clear, albedo_clear, depth_clear]
    }

    unsafe fn bind_pipeline_and_state(&self, command_buffer: vk::CommandBuffer) {
        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline.pipeline,
        );

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(self.gbuffer.width as f32)
            .height(self.gbuffer.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(vk::Extent2D {
                width: self.gbuffer.width,
                height: self.gbuffer.height,
            });

        self.device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        self.device.cmd_set_scissor(command_buffer, 0, &[scissor]);
    }

    unsafe fn draw_meshes(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        for i in 0..self.descriptor_set.rrdata.len() {
            let rrdata = &self.descriptor_set.rrdata[i];

            self.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[rrdata.vertex_buffer.buffer],
                &[0],
            );

            self.device.cmd_bind_index_buffer(
                command_buffer,
                rrdata.index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );

            let swapchain_images_len = self.descriptor_set.descriptor_sets.len() /
                self.descriptor_set.rrdata.len().max(1);
            let descriptor_set_index = i * swapchain_images_len + image_index;

            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline_layout,
                0,
                &[self.descriptor_set.descriptor_sets[descriptor_set_index]],
                &[],
            );

            self.device.cmd_draw_indexed(
                command_buffer,
                rrdata.index_buffer.indices,
                1,
                0,
                0,
                0,
            );
        }

        Ok(())
    }
}
