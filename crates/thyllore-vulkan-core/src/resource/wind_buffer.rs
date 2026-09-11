use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::resource::hdr_buffer::HDR_FORMAT;
use crate::resource::image::{
    create_image, create_image_3d, create_image_view, create_image_view_3d,
};
use thyllore_effect_core::{
    WIND_SHADOW_VOLUME_HEIGHT, WIND_SHADOW_VOLUME_RADIAL, WIND_SHADOW_VOLUME_SLOTS,
    WIND_SHADOW_VOLUME_THETA,
};

pub const WIND_SHADOW_VOLUME_FORMAT: vk::Format = vk::Format::R16G16_SFLOAT;

/// Render pass and framebuffer that blend the wind resolve pass onto the HDR color image,
/// the half resolution intermediate target used by the upsampled resolve path, and the
/// shadow volume baked once per frame for every instance slot.
#[derive(Clone, Debug, Default)]
pub struct WindBuffer {
    pub render_pass: vk::RenderPass,
    pub framebuffer: vk::Framebuffer,
    pub half_render_pass: vk::RenderPass,
    pub half_framebuffer: vk::Framebuffer,
    pub half_color_image: vk::Image,
    pub half_color_image_memory: vk::DeviceMemory,
    pub half_color_image_view: vk::ImageView,
    pub shadow_volume_image: vk::Image,
    pub shadow_volume_memory: vk::DeviceMemory,
    pub shadow_volume_view: vk::ImageView,
    pub shadow_volume_sampler: vk::Sampler,
    pub width: u32,
    pub height: u32,
}

pub fn wind_shadow_volume_extent() -> vk::Extent3D {
    vk::Extent3D {
        width: WIND_SHADOW_VOLUME_RADIAL * WIND_SHADOW_VOLUME_SLOTS,
        height: WIND_SHADOW_VOLUME_HEIGHT,
        depth: WIND_SHADOW_VOLUME_THETA,
    }
}

/// Radius and height clamp, the angle wraps around the seam at theta = -pi.
unsafe fn create_shadow_volume_sampler(rrdevice: &RRDevice) -> Result<vk::Sampler> {
    let info = vk::SamplerCreateInfo::builder()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .anisotropy_enable(false)
        .max_anisotropy(1.0)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(0.0);
    Ok(rrdevice.device.create_sampler(&info, None)?)
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

        let (shadow_volume_image, shadow_volume_memory) = create_image_3d(
            instance,
            rrdevice,
            wind_shadow_volume_extent(),
            WIND_SHADOW_VOLUME_FORMAT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
        )?;
        let shadow_volume_view =
            create_image_view_3d(rrdevice, shadow_volume_image, WIND_SHADOW_VOLUME_FORMAT)?;
        let shadow_volume_sampler = create_shadow_volume_sampler(rrdevice)?;

        log!("Created wind buffer: {}x{}", width, height);
        Ok(Self {
            render_pass,
            framebuffer,
            half_render_pass,
            half_framebuffer,
            half_color_image,
            half_color_image_memory,
            half_color_image_view,
            shadow_volume_image,
            shadow_volume_memory,
            shadow_volume_view,
            shadow_volume_sampler,
            width,
            height,
        })
    }

    /// Shared by the full and half resolution passes so pipelines built against one
    /// stay render-pass compatible with the other (dependencies must match exactly).
    /// The incoming edge covers both prior uses of the target: an HDR colour write
    /// (full pass, LOAD) and the previous frame's upsample read (half pass, CLEAR).
    fn subpass_dependencies() -> [vk::SubpassDependency; 2] {
        let dependency_in = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    | vk::PipelineStageFlags::FRAGMENT_SHADER,
            )
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::SHADER_READ)
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

        [dependency_in, dependency_out]
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

        let attachments = [color_attachment];
        let subpasses = [subpass];
        let dependencies = Self::subpass_dependencies();
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

        let attachments = [color_attachment];
        let subpasses = [subpass];
        let dependencies = Self::subpass_dependencies();
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
        if self.shadow_volume_sampler != vk::Sampler::null() {
            device.destroy_sampler(self.shadow_volume_sampler, None);
            self.shadow_volume_sampler = vk::Sampler::null();
        }
        if self.shadow_volume_view != vk::ImageView::null() {
            device.destroy_image_view(self.shadow_volume_view, None);
            self.shadow_volume_view = vk::ImageView::null();
        }
        if self.shadow_volume_image != vk::Image::null() {
            device.destroy_image(self.shadow_volume_image, None);
            self.shadow_volume_image = vk::Image::null();
        }
        if self.shadow_volume_memory != vk::DeviceMemory::null() {
            device.free_memory(self.shadow_volume_memory, None);
            self.shadow_volume_memory = vk::DeviceMemory::null();
        }
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
