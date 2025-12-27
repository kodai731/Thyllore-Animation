use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
use super::gbuffer::RRGBuffer;
use rust_rendering::vulkanr::pipeline::RRPipeline;
use rust_rendering::vulkanr::descriptor::RRRayQueryDescriptorSet;
use rust_rendering::vulkanr::core::Device;

pub struct RayQueryPass<'a> {
    gbuffer: &'a RRGBuffer,
    pipeline: &'a RRPipeline,
    descriptor_set: &'a RRRayQueryDescriptorSet,
    device: &'a Device,
}

impl<'a> RayQueryPass<'a> {
    pub fn new(app: &'a App) -> Result<Self> {
        let gbuffer = app.data.gbuffer.as_ref()
            .ok_or_else(|| anyhow!("G-Buffer not initialized"))?;
        let pipeline = app.data.ray_query_pipeline.as_ref()
            .ok_or_else(|| anyhow!("Ray Query pipeline not initialized"))?;
        let descriptor_set = app.data.ray_query_descriptor.as_ref()
            .ok_or_else(|| anyhow!("Ray Query descriptor set not initialized"))?;

        Ok(Self {
            gbuffer,
            pipeline,
            descriptor_set,
            device: &app.rrdevice.device,
        })
    }

    pub unsafe fn record(&self, command_buffer: vk::CommandBuffer) -> Result<()> {
        self.insert_pre_compute_barriers(command_buffer);
        self.dispatch_compute(command_buffer);
        self.insert_post_compute_barriers(command_buffer);

        Ok(())
    }

    unsafe fn insert_pre_compute_barriers(&self, command_buffer: vk::CommandBuffer) {
        let image_barriers = [
            vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .old_layout(vk::ImageLayout::GENERAL)
                .new_layout(vk::ImageLayout::GENERAL)
                .image(self.gbuffer.position_image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .build(),
            vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .old_layout(vk::ImageLayout::GENERAL)
                .new_layout(vk::ImageLayout::GENERAL)
                .image(self.gbuffer.normal_image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .build(),
            vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::SHADER_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::GENERAL)
                .image(self.gbuffer.shadow_mask_image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .build(),
        ];

        self.device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &image_barriers,
        );
    }

    unsafe fn dispatch_compute(&self, command_buffer: vk::CommandBuffer) {
        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            self.pipeline.pipeline,
        );

        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            self.pipeline.pipeline_layout,
            0,
            &[self.descriptor_set.descriptor_set],
            &[],
        );

        let group_count_x = (self.gbuffer.width + 15) / 16;
        let group_count_y = (self.gbuffer.height + 15) / 16;
        self.device.cmd_dispatch(command_buffer, group_count_x, group_count_y, 1);
    }

    unsafe fn insert_post_compute_barriers(&self, command_buffer: vk::CommandBuffer) {
        let shadow_barrier = vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::GENERAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image(self.gbuffer.shadow_mask_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build();

        self.device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[shadow_barrier],
        );
    }
}
