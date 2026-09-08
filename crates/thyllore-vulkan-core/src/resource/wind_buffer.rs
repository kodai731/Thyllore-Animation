use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::resource::hdr_buffer::HDR_FORMAT;
use crate::resource::image::{create_image, create_image_view};

/// Render pass and framebuffer that blend the wind resolve pass onto the HDR color image,
/// plus the half resolution intermediate target used by the upsampled resolve path.
#[derive(Clone, Debug, Default)]
pub struct WindBuffer {
    pub render_pass: vk::RenderPass,
    pub framebuffer: vk::Framebuffer,
    pub half_render_pass: vk::RenderPass,
    pub half_framebuffer: vk::Framebuffer,
    pub half_color_image: vk::Image,
    pub half_color_image_memory: vk::DeviceMemory,
    pub half_color_image_view: vk::ImageView,
    pub width: u32,
    pub height: u32,
}

impl WindBuffer {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        width: u32,
        height: u32,
        hdr_image_view: vk::ImageView,
    ) -> Result<Self> {
        let render_pass = Self::create_render_pass(rrdevice)?;
        let attachments = [hdr_image_view];
        let framebuffer_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(width)
            .height(height)
            .layers(1);
        let framebuffer = rrdevice
            .device
            .create_framebuffer(&framebuffer_info, None)?;

        let half_extent = half_extent(width, height);
        let (half_color_image, half_color_image_memory) = create_image(
            instance,
            rrdevice,
            half_extent.width,
            half_extent.height,
            1,
            vk::SampleCountFlags::_1,
            HDR_FORMAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        let half_color_image_view = create_image_view(
            rrdevice,
            half_color_image,
            HDR_FORMAT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let half_render_pass = Self::create_half_render_pass(rrdevice)?;
        let half_attachments = [half_color_image_view];
        let half_framebuffer_info = vk::FramebufferCreateInfo::builder()
            .render_pass(half_render_pass)
            .attachments(&half_attachments)
            .width(half_extent.width)
            .height(half_extent.height)
            .layers(1);
        let half_framebuffer = rrdevice
            .device
            .create_framebuffer(&half_framebuffer_info, None)?;

        log!("Created wind buffer: {}x{}", width, height);
        Ok(Self {
            render_pass,
            framebuffer,
            half_render_pass,
            half_framebuffer,
            half_color_image,
            half_color_image_memory,
            half_color_image_view,
            width,
            height,
        })
    }

    unsafe fn create_render_pass(rrdevice: &RRDevice) -> Result<vk::RenderPass> {
        let color_attachment = vk::AttachmentDescription::builder()
            .format(HDR_FORMAT)
            .samples(vk::SampleCountFlags::_1)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build();

        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();
        let color_attachments = [color_attachment_ref];

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments);

        let dependency_in = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            )
            .build();

        let dependency_out = vk::SubpassDependency::builder()
            .src_subpass(0)
            .dst_subpass(vk::SUBPASS_EXTERNAL)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .build();

        let attachments = [color_attachment];
        let subpasses = [subpass];
        let dependencies = [dependency_in, dependency_out];
        let info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        Ok(rrdevice.device.create_render_pass(&info, None)?)
    }

    unsafe fn create_half_render_pass(rrdevice: &RRDevice) -> Result<vk::RenderPass> {
        let color_attachment = vk::AttachmentDescription::builder()
            .format(HDR_FORMAT)
            .samples(vk::SampleCountFlags::_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build();

        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();
        let color_attachments = [color_attachment_ref];

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments);

        let dependency_in = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .src_access_mask(vk::AccessFlags::SHADER_READ)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .build();

        let dependency_out = vk::SubpassDependency::builder()
            .src_subpass(0)
            .dst_subpass(vk::SUBPASS_EXTERNAL)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .build();

        let attachments = [color_attachment];
        let subpasses = [subpass];
        let dependencies = [dependency_in, dependency_out];
        let info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        Ok(rrdevice.device.create_render_pass(&info, None)?)
    }

    pub unsafe fn resize(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        new_width: u32,
        new_height: u32,
        hdr_image_view: vk::ImageView,
    ) -> Result<()> {
        self.destroy(&rrdevice.device);
        *self = Self::new(instance, rrdevice, new_width, new_height, hdr_image_view)?;
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        if self.half_framebuffer != vk::Framebuffer::null() {
            device.destroy_framebuffer(self.half_framebuffer, None);
            self.half_framebuffer = vk::Framebuffer::null();
        }
        if self.half_render_pass != vk::RenderPass::null() {
            device.destroy_render_pass(self.half_render_pass, None);
            self.half_render_pass = vk::RenderPass::null();
        }
        if self.half_color_image_view != vk::ImageView::null() {
            device.destroy_image_view(self.half_color_image_view, None);
            self.half_color_image_view = vk::ImageView::null();
        }
        if self.half_color_image != vk::Image::null() {
            device.destroy_image(self.half_color_image, None);
            self.half_color_image = vk::Image::null();
        }
        if self.half_color_image_memory != vk::DeviceMemory::null() {
            device.free_memory(self.half_color_image_memory, None);
            self.half_color_image_memory = vk::DeviceMemory::null();
        }
        if self.framebuffer != vk::Framebuffer::null() {
            device.destroy_framebuffer(self.framebuffer, None);
            self.framebuffer = vk::Framebuffer::null();
        }
        if self.render_pass != vk::RenderPass::null() {
            device.destroy_render_pass(self.render_pass, None);
            self.render_pass = vk::RenderPass::null();
        }
    }

    pub fn extent(&self) -> vk::Extent2D {
        vk::Extent2D {
            width: self.width,
            height: self.height,
        }
    }

    pub fn half_extent(&self) -> vk::Extent2D {
        half_extent(self.width, self.height)
    }
}

fn half_extent(width: u32, height: u32) -> vk::Extent2D {
    vk::Extent2D {
        width: (width / 2).max(1),
        height: (height / 2).max(1),
    }
}

impl Drop for WindBuffer {
    fn drop(&mut self) {
        if self.render_pass != vk::RenderPass::null() {
            log_warn!("WindBuffer dropped without calling destroy()");
        }
    }
}
