use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
use crate::vulkanr::core::Device;
use crate::vulkanr::descriptor::RRBloomDescriptorSets;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::resource::BloomChain;

#[repr(C)]
#[derive(Clone, Copy)]
struct BloomDownsamplePushConstants {
    threshold: f32,
    knee: f32,
    is_first_pass: i32,
}

pub struct BloomPass<'a> {
    downsample_pipeline: &'a RRPipeline,
    upsample_pipeline: &'a RRPipeline,
    bloom_descriptors: &'a RRBloomDescriptorSets,
    bloom_chain: &'a BloomChain,
    threshold: f32,
    knee: f32,
    device: &'a Device,
}

impl<'a> BloomPass<'a> {
    pub fn new(app: &'a App) -> Result<Self> {
        let downsample_pipeline = app
            .data
            .raytracing
            .bloom_downsample_pipeline
            .as_ref()
            .ok_or_else(|| anyhow!("Bloom downsample pipeline not initialized"))?;

        let upsample_pipeline = app
            .data
            .raytracing
            .bloom_upsample_pipeline
            .as_ref()
            .ok_or_else(|| anyhow!("Bloom upsample pipeline not initialized"))?;

        let bloom_descriptors = app
            .data
            .raytracing
            .bloom_descriptors
            .as_ref()
            .ok_or_else(|| anyhow!("Bloom descriptors not initialized"))?;

        let bloom_chain = app
            .data
            .viewport
            .bloom_chain
            .as_ref()
            .ok_or_else(|| anyhow!("Bloom chain not initialized"))?;

        let bloom_settings = app
            .data
            .ecs_world
            .get_resource::<crate::ecs::resource::BloomSettings>();

        let (threshold, knee) = match bloom_settings {
            Some(bs) => (bs.threshold, bs.knee),
            None => (1.0, 0.5),
        };

        Ok(Self {
            downsample_pipeline,
            upsample_pipeline,
            bloom_descriptors,
            bloom_chain,
            threshold,
            knee,
            device: &app.rrdevice.device,
        })
    }

    pub unsafe fn record(&self, command_buffer: vk::CommandBuffer) -> Result<()> {
        self.record_downsample_passes(command_buffer)?;
        self.record_upsample_passes(command_buffer)?;
        Ok(())
    }

    unsafe fn record_downsample_passes(&self, command_buffer: vk::CommandBuffer) -> Result<()> {
        let mip_count = self.bloom_chain.mip_levels.len();

        for i in 0..mip_count {
            let mip = &self.bloom_chain.mip_levels[i];
            let extent = vk::Extent2D {
                width: mip.width,
                height: mip.height,
            };

            self.begin_downsample_render_pass(command_buffer, mip.framebuffer, extent);

            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.downsample_pipeline.pipeline,
            );

            self.set_viewport_and_scissor(command_buffer, extent);

            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.downsample_pipeline.pipeline_layout,
                0,
                &[self.bloom_descriptors.downsample_sets[i]],
                &[],
            );

            let push_constants = BloomDownsamplePushConstants {
                threshold: self.threshold,
                knee: self.knee,
                is_first_pass: if i == 0 { 1 } else { 0 },
            };

            let push_bytes = std::slice::from_raw_parts(
                &push_constants as *const BloomDownsamplePushConstants as *const u8,
                std::mem::size_of::<BloomDownsamplePushConstants>(),
            );

            self.device.cmd_push_constants(
                command_buffer,
                self.downsample_pipeline.pipeline_layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                push_bytes,
            );

            self.device.cmd_draw(command_buffer, 3, 1, 0, 0);

            self.device.cmd_end_render_pass(command_buffer);
        }

        Ok(())
    }

    unsafe fn record_upsample_passes(&self, command_buffer: vk::CommandBuffer) -> Result<()> {
        let mip_count = self.bloom_chain.mip_levels.len();
        if mip_count < 2 {
            return Ok(());
        }

        for (pass_idx, target_mip_idx) in (0..mip_count - 1).rev().enumerate() {
            let mip = &self.bloom_chain.mip_levels[target_mip_idx];
            let extent = vk::Extent2D {
                width: mip.width,
                height: mip.height,
            };

            self.transition_to_color_attachment(command_buffer, mip.image);

            self.begin_upsample_render_pass(command_buffer, mip.framebuffer, extent);

            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.upsample_pipeline.pipeline,
            );

            self.set_viewport_and_scissor(command_buffer, extent);

            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.upsample_pipeline.pipeline_layout,
                0,
                &[self.bloom_descriptors.upsample_sets[pass_idx]],
                &[],
            );

            self.device.cmd_draw(command_buffer, 3, 1, 0, 0);

            self.device.cmd_end_render_pass(command_buffer);
        }

        Ok(())
    }

    unsafe fn transition_to_color_attachment(
        &self,
        command_buffer: vk::CommandBuffer,
        image: vk::Image,
    ) {
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::SHADER_READ)
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            );

        self.device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[barrier],
        );
    }

    unsafe fn begin_downsample_render_pass(
        &self,
        command_buffer: vk::CommandBuffer,
        framebuffer: vk::Framebuffer,
        extent: vk::Extent2D,
    ) {
        let clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.bloom_chain.downsample_render_pass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent,
            })
            .clear_values(std::slice::from_ref(&clear_value));

        self.device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_info,
            vk::SubpassContents::INLINE,
        );
    }

    unsafe fn begin_upsample_render_pass(
        &self,
        command_buffer: vk::CommandBuffer,
        framebuffer: vk::Framebuffer,
        extent: vk::Extent2D,
    ) {
        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.bloom_chain.upsample_render_pass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent,
            });

        self.device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_info,
            vk::SubpassContents::INLINE,
        );
    }

    unsafe fn set_viewport_and_scissor(
        &self,
        command_buffer: vk::CommandBuffer,
        extent: vk::Extent2D,
    ) {
        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(extent);

        self.device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        self.device.cmd_set_scissor(command_buffer, 0, &[scissor]);
    }
}
