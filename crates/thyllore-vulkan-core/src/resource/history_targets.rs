use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::command::{begin_single_time_commands, end_single_time_commands};
use crate::core::RRDevice;
use crate::resource::gpu_resource::GpuResource;
use crate::resource::hdr_buffer::HDR_FORMAT;
use crate::resource::render_target_storage::{RenderTargetKey, RenderTargetStorage};

const COLOR_RANGE: vk::ImageSubresourceRange = vk::ImageSubresourceRange {
    aspect_mask: vk::ImageAspectFlags::COLOR,
    base_mip_level: 0,
    level_count: 1,
    base_array_layer: 0,
    layer_count: 1,
};

/// What differs between two effects that both shade onto the HDR image while writing a
/// temporal history: the storage keys, the history format, extra usage bits, the sampler,
/// whether the history attachment is cleared, and whether the scene depth takes part.
#[derive(Clone, Copy, Debug)]
pub struct HistoryTargetsDesc {
    pub keys: [RenderTargetKey; 2],
    pub format: vk::Format,
    pub usage: vk::ImageUsageFlags,
    pub sampler: vk::SamplerCreateInfo,
    pub history_load_op: vk::AttachmentLoadOp,
    pub depth_view: Option<vk::ImageView>,
}

/// A history ping-pong pair from `RenderTargetStorage` plus the render pass and the two
/// framebuffers that shade onto the HDR image while writing history[i] and sampling
/// history[1 - i]. The images belong to the storage; `destroy` only owns the pass objects.
#[derive(Clone, Debug, Default)]
pub struct HistoryTargets {
    pub images: [vk::Image; 2],
    pub views: [vk::ImageView; 2],
    pub sampler: vk::Sampler,
    pub render_pass: vk::RenderPass,
    pub framebuffers: [vk::Framebuffer; 2],
    pub width: u32,
    pub height: u32,
}

impl HistoryTargets {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        storage: &mut RenderTargetStorage,
        command_pool: vk::CommandPool,
        width: u32,
        height: u32,
        hdr_view: vk::ImageView,
        desc: HistoryTargetsDesc,
    ) -> Result<Self> {
        storage.ensure_extent(&rrdevice.device, width, height);

        let usage = vk::ImageUsageFlags::COLOR_ATTACHMENT
            | vk::ImageUsageFlags::SAMPLED
            | vk::ImageUsageFlags::TRANSFER_DST
            | desc.usage;
        let current = *storage.ensure(
            instance,
            rrdevice,
            desc.keys[0],
            desc.format,
            usage,
            Some(desc.sampler),
        )?;
        let previous =
            *storage.ensure(instance, rrdevice, desc.keys[1], desc.format, usage, None)?;
        let images = [current.image, previous.image];
        let views = [current.view, previous.view];

        clear_to_shader_read_only(rrdevice, command_pool, &images)?;

        let render_pass = create_history_render_pass(rrdevice, &desc)?;
        let mut framebuffers = [vk::Framebuffer::null(); 2];
        for (i, framebuffer) in framebuffers.iter_mut().enumerate() {
            let mut attachments = vec![hdr_view, views[i]];
            attachments.extend(desc.depth_view);
            let info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(width)
                .height(height)
                .layers(1);
            *framebuffer = rrdevice.device.create_framebuffer(&info, None)?;
        }

        Ok(Self {
            images,
            views,
            sampler: current.sampler,
            render_pass,
            framebuffers,
            width,
            height,
        })
    }

    pub fn extent(&self) -> vk::Extent2D {
        vk::Extent2D {
            width: self.width,
            height: self.height,
        }
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        for framebuffer in &mut self.framebuffers {
            if *framebuffer != vk::Framebuffer::null() {
                device.destroy_framebuffer(*framebuffer, None);
                *framebuffer = vk::Framebuffer::null();
            }
        }
        if self.render_pass != vk::RenderPass::null() {
            device.destroy_render_pass(self.render_pass, None);
            self.render_pass = vk::RenderPass::null();
        }

        self.images = [vk::Image::null(); 2];
        self.views = [vk::ImageView::null(); 2];
        self.sampler = vk::Sampler::null();
    }

    fn holds_pass_objects(&self) -> bool {
        self.framebuffers
            .iter()
            .any(|framebuffer| *framebuffer != vk::Framebuffer::null())
    }
}

impl Drop for HistoryTargets {
    fn drop(&mut self) {
        if self.holds_pass_objects() {
            log_warn!("HistoryTargets dropped without calling destroy()");
        }
    }
}

impl GpuResource for HistoryTargets {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device);
    }
}

unsafe fn clear_to_shader_read_only(
    rrdevice: &RRDevice,
    command_pool: vk::CommandPool,
    images: &[vk::Image; 2],
) -> Result<()> {
    let cmd = begin_single_time_commands(rrdevice, command_pool)?;
    for &image in images {
        let to_transfer = vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .image(image)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(COLOR_RANGE)
            .build();
        rrdevice.device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[to_transfer],
        );

        let clear_value = vk::ClearColorValue {
            float32: [0.0f32; 4],
        };
        rrdevice.device.cmd_clear_color_image(
            cmd,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &clear_value,
            &[COLOR_RANGE],
        );

        let to_sampled = vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .image(image)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(COLOR_RANGE)
            .build();
        rrdevice.device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[to_sampled],
        );
    }
    end_single_time_commands(rrdevice, rrdevice.graphics_queue, command_pool, cmd)
}

unsafe fn create_history_render_pass(
    rrdevice: &RRDevice,
    desc: &HistoryTargetsDesc,
) -> Result<vk::RenderPass> {
    let hdr_attachment = vk::AttachmentDescription::builder()
        .format(HDR_FORMAT)
        .samples(vk::SampleCountFlags::_1)
        .load_op(vk::AttachmentLoadOp::LOAD)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .build();
    let history_attachment = vk::AttachmentDescription::builder()
        .format(desc.format)
        .samples(vk::SampleCountFlags::_1)
        .load_op(desc.history_load_op)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .build();
    let depth_attachment = vk::AttachmentDescription::builder()
        .format(vk::Format::D32_SFLOAT)
        .samples(vk::SampleCountFlags::_1)
        .load_op(vk::AttachmentLoadOp::LOAD)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL)
        .build();

    let color_refs = [
        vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build(),
        vk::AttachmentReference::builder()
            .attachment(1)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build(),
    ];
    let depth_ref = vk::AttachmentReference::builder()
        .attachment(2)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .build();

    let mut attachments = vec![hdr_attachment, history_attachment];
    let mut subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_refs);
    if desc.depth_view.is_some() {
        attachments.push(depth_attachment);
        subpass = subpass.depth_stencil_attachment(&depth_ref);
    }
    let subpasses = [subpass];
    let dependencies = history_pass_dependencies(desc.depth_view.is_some());

    let info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);
    Ok(rrdevice.device.create_render_pass(&info, None)?)
}

fn history_pass_dependencies(with_depth: bool) -> [vk::SubpassDependency; 2] {
    let fragment_tests =
        vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS;
    let (in_stages, in_dst_access, out_src_stages, out_src_access, out_dst_stages) = if with_depth {
        (
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | fragment_tests,
            vk::AccessFlags::COLOR_ATTACHMENT_READ
                | vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
    } else {
        (
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        )
    };

    let dependency_in = vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(in_stages)
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_stage_mask(in_stages)
        .dst_access_mask(in_dst_access)
        .build();
    let dependency_out = vk::SubpassDependency::builder()
        .src_subpass(0)
        .dst_subpass(vk::SUBPASS_EXTERNAL)
        .src_stage_mask(out_src_stages)
        .src_access_mask(out_src_access)
        .dst_stage_mask(out_dst_stages)
        .dst_access_mask(vk::AccessFlags::SHADER_READ)
        .build();
    [dependency_in, dependency_out]
}
