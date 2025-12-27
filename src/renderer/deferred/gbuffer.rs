use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
use rust_rendering::vulkanr::render::RRGBuffer;
use rust_rendering::vulkanr::pipeline::RRPipeline;
use rust_rendering::vulkanr::descriptor::RRDescriptorSet;
use rust_rendering::vulkanr::core::Device;

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
