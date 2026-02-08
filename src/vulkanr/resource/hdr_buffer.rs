use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::resource::image::{create_image, create_image_view};

pub const HDR_FORMAT: vk::Format = vk::Format::R16G16B16A16_SFLOAT;

#[derive(Clone, Debug, Default)]
pub struct HdrBuffer {
    pub color_image: vk::Image,
    pub color_image_memory: vk::DeviceMemory,
    pub color_image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub framebuffer: vk::Framebuffer,
    pub render_pass: vk::RenderPass,
    pub width: u32,
    pub height: u32,
}

impl HdrBuffer {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let (color_image, color_image_memory) = create_image(
            instance,
            rrdevice,
            width,
            height,
            1,
            vk::SampleCountFlags::_1,
            HDR_FORMAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let color_image_view = create_image_view(
            rrdevice,
            color_image,
            HDR_FORMAT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let render_pass = Self::create_render_pass(rrdevice)?;

        let attachments = [color_image_view];
        let framebuffer_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(width)
            .height(height)
            .layers(1);
        let framebuffer = rrdevice.device.create_framebuffer(&framebuffer_info, None)?;

        let sampler = Self::create_sampler(&rrdevice.device)?;

        crate::log!(
            "Created HDR buffer: {}x{} format {:?}",
            width, height, HDR_FORMAT
        );

        Ok(Self {
            color_image,
            color_image_memory,
            color_image_view,
            sampler,
            framebuffer,
            render_pass,
            width,
            height,
        })
    }

    unsafe fn create_render_pass(rrdevice: &RRDevice) -> Result<vk::RenderPass> {
        let color_attachment = vk::AttachmentDescription::builder()
            .format(HDR_FORMAT)
            .samples(vk::SampleCountFlags::_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let color_attachments = [color_attachment_ref];

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments);

        let dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .src_access_mask(vk::AccessFlags::SHADER_READ)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        let dependency_out = vk::SubpassDependency::builder()
            .src_subpass(0)
            .dst_subpass(vk::SUBPASS_EXTERNAL)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .dst_access_mask(vk::AccessFlags::SHADER_READ);

        let attachments = [color_attachment];
        let subpasses = [subpass];
        let dependencies = [dependency, dependency_out];

        let info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        let render_pass = rrdevice.device.create_render_pass(&info, None)?;
        Ok(render_pass)
    }

    unsafe fn create_sampler(device: &vulkanalia::Device) -> Result<vk::Sampler> {
        let sampler_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .anisotropy_enable(false)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(1.0);

        let sampler = device.create_sampler(&sampler_info, None)?;
        Ok(sampler)
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

        self.destroy(&rrdevice.device);

        let new_buf = Self::new(instance, rrdevice, new_width, new_height)?;
        *self = new_buf;

        crate::log!("Resized HDR buffer to: {}x{}", new_width, new_height);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        device.destroy_sampler(self.sampler, None);
        device.destroy_framebuffer(self.framebuffer, None);
        device.destroy_render_pass(self.render_pass, None);
        device.destroy_image_view(self.color_image_view, None);
        device.destroy_image(self.color_image, None);
        device.free_memory(self.color_image_memory, None);

        crate::log!("Destroyed HDR buffer");
    }

    pub fn extent(&self) -> vk::Extent2D {
        vk::Extent2D {
            width: self.width,
            height: self.height,
        }
    }
}
