use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::resource::hdr_buffer::HDR_FORMAT;

#[derive(Clone, Debug, Default)]
pub struct OnionSkinPassResources {
    pub render_pass: vk::RenderPass,
    pub framebuffer: vk::Framebuffer,
    pub pipeline: RRPipeline,
    pub width: u32,
    pub height: u32,
}

impl OnionSkinPassResources {
    pub unsafe fn create_render_pass(rrdevice: &RRDevice) -> Result<vk::RenderPass> {
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

        let render_pass = rrdevice.device.create_render_pass(&info, None)?;
        Ok(render_pass)
    }

    pub unsafe fn create_framebuffer(
        rrdevice: &RRDevice,
        render_pass: vk::RenderPass,
        hdr_color_image_view: vk::ImageView,
        width: u32,
        height: u32,
    ) -> Result<vk::Framebuffer> {
        let attachments = [hdr_color_image_view];

        let info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(width)
            .height(height)
            .layers(1);

        let framebuffer = rrdevice.device.create_framebuffer(&info, None)?;
        Ok(framebuffer)
    }

    pub unsafe fn destroy(&self, device: &vulkanalia::Device) {
        device.destroy_framebuffer(self.framebuffer, None);
        self.pipeline.destroy(device);
        device.destroy_render_pass(self.render_pass, None);
        crate::log!("Destroyed onion skin pass resources");
    }

    pub unsafe fn recreate_framebuffer(
        &mut self,
        rrdevice: &RRDevice,
        hdr_color_image_view: vk::ImageView,
        width: u32,
        height: u32,
    ) -> Result<()> {
        rrdevice.device.destroy_framebuffer(self.framebuffer, None);

        self.framebuffer = Self::create_framebuffer(
            rrdevice,
            self.render_pass,
            hdr_color_image_view,
            width,
            height,
        )?;
        self.width = width;
        self.height = height;

        crate::log!("Recreated onion skin framebuffer: {}x{}", width, height);
        Ok(())
    }
}
