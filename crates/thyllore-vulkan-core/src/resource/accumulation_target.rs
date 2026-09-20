use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::command::{begin_single_time_commands, end_single_time_commands};
use crate::core::RRDevice;
use crate::resource::gpu_resource::GpuResource;
use crate::resource::render_target_storage::{RenderTargetKey, RenderTargetStorage};

/// A storage image from `RenderTargetStorage` that compute passes accumulate into, kept in
/// GENERAL layout for its whole life. The storage owns the image; this only holds handles.
#[derive(Clone, Copy, Debug, Default)]
pub struct AccumulationTarget {
    pub image: vk::Image,
    pub view: vk::ImageView,
}

impl AccumulationTarget {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        storage: &mut RenderTargetStorage,
        command_pool: vk::CommandPool,
        key: RenderTargetKey,
        format: vk::Format,
    ) -> Result<Self> {
        let usage = vk::ImageUsageFlags::STORAGE
            | vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::TRANSFER_SRC;
        let entry = *storage.ensure(instance, rrdevice, key, format, usage, None)?;

        let cmd = begin_single_time_commands(rrdevice, command_pool)?;
        let to_general = vk::ImageMemoryBarrier::builder()
            .image(entry.image)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::SHADER_WRITE | vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::GENERAL)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build();
        rrdevice.device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[to_general],
        );
        end_single_time_commands(rrdevice, rrdevice.graphics_queue, command_pool, cmd)?;

        Ok(Self {
            image: entry.image,
            view: entry.view,
        })
    }
}

impl GpuResource for AccumulationTarget {
    unsafe fn destroy_gpu(&mut self, _rrdevice: &RRDevice) {
        self.image = vk::Image::null();
        self.view = vk::ImageView::null();
    }
}
