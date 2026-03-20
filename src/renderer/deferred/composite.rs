use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use super::OverlayRenderer;
use crate::app::App;
use crate::debugview::DebugViewMode;
use crate::vulkanr::core::Device;
use crate::vulkanr::descriptor::RRCompositeDescriptorSet;
use crate::vulkanr::pipeline::RRPipeline;

pub struct CompositePass<'a> {
    app: &'a App,
    composite_pipeline: &'a RRPipeline,
    composite_descriptor: &'a RRCompositeDescriptorSet,
    device: &'a Device,
    swapchain_extent: vk::Extent2D,
    debug_view_mode: DebugViewMode,
}

impl<'a> CompositePass<'a> {
    pub fn new(app: &'a App) -> Result<Self> {
        let composite_pipeline = app
            .data
            .raytracing
            .composite_pipeline
            .as_ref()
            .ok_or_else(|| anyhow!("Composite pipeline not initialized"))?;
        let composite_descriptor = app
            .data
            .raytracing
            .composite_descriptor
            .as_ref()
            .ok_or_else(|| anyhow!("Composite descriptor set not initialized"))?;

        Ok(Self {
            app,
            composite_pipeline,
            composite_descriptor,
            device: &app.rrdevice.device,
            swapchain_extent: app
                .resource::<crate::vulkanr::context::SwapchainState>()
                .swapchain
                .swapchain_extent,
            debug_view_mode: app
                .resource::<crate::debugview::DebugViewState>()
                .debug_view_mode,
        })
    }

    pub fn new_for_offscreen(app: &'a App, offscreen_extent: vk::Extent2D) -> Result<Self> {
        let composite_pipeline = app
            .data
            .raytracing
            .composite_pipeline
            .as_ref()
            .ok_or_else(|| anyhow!("Composite pipeline not initialized"))?;
        let composite_descriptor = app
            .data
            .raytracing
            .composite_descriptor
            .as_ref()
            .ok_or_else(|| anyhow!("Composite descriptor set not initialized"))?;

        Ok(Self {
            app,
            composite_pipeline,
            composite_descriptor,
            device: &app.rrdevice.device,
            swapchain_extent: offscreen_extent,
            debug_view_mode: app
                .resource::<crate::debugview::DebugViewState>()
                .debug_view_mode,
        })
    }

    pub unsafe fn record(
        &self,
        command_buffer: vk::CommandBuffer,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        image_index: usize,
    ) -> Result<()> {
        self.begin_render_pass(command_buffer, render_pass, framebuffer, 2);
        self.draw_composite(command_buffer)?;
        OverlayRenderer::new(self.app).draw_all_overlays(command_buffer, image_index)?;

        Ok(())
    }

    pub unsafe fn record_to_offscreen(
        &self,
        command_buffer: vk::CommandBuffer,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        image_index: usize,
    ) -> Result<()> {
        self.begin_render_pass(command_buffer, render_pass, framebuffer, 3);
        self.draw_composite(command_buffer)?;
        OverlayRenderer::new(self.app).draw_all_overlays(command_buffer, image_index)?;
        self.device.cmd_end_render_pass(command_buffer);

        Ok(())
    }

    pub unsafe fn record_to_hdr(
        &self,
        command_buffer: vk::CommandBuffer,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
    ) -> Result<()> {
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(self.swapchain_extent);

        let color_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let clear_values = [color_clear_value];

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

        self.draw_composite(command_buffer)?;

        self.device.cmd_end_render_pass(command_buffer);

        Ok(())
    }

    unsafe fn begin_render_pass(
        &self,
        command_buffer: vk::CommandBuffer,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        attachment_count: usize,
    ) {
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(self.swapchain_extent);

        let color_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };
        let depth_clear_value = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 0.0,
                stencil: 0,
            },
        };

        let clear_values: Vec<vk::ClearValue> = if attachment_count == 3 {
            vec![color_clear_value, depth_clear_value, color_clear_value]
        } else {
            vec![color_clear_value, depth_clear_value]
        };

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
    }

    unsafe fn draw_composite(&self, command_buffer: vk::CommandBuffer) -> Result<()> {
        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.composite_pipeline.pipeline,
        );

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(self.swapchain_extent.width as f32)
            .height(self.swapchain_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(self.swapchain_extent);

        self.device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        self.device.cmd_set_scissor(command_buffer, 0, &[scissor]);

        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.composite_pipeline.pipeline_layout,
            0,
            &[self.composite_descriptor.descriptor_set],
            &[],
        );

        let debug_view_mode_value = match self.debug_view_mode {
            DebugViewMode::Final => 0,
            DebugViewMode::Position => 1,
            DebugViewMode::Normal => 2,
            DebugViewMode::ShadowMask => 3,
            DebugViewMode::NdotL => 4,
            DebugViewMode::LightDirection => 5,
            DebugViewMode::ViewDepth => 6,
            DebugViewMode::ObjectID => 7,
            DebugViewMode::SelectionView => 8,
            DebugViewMode::SelectionUBO => 9,
        };

        let push_constants = [debug_view_mode_value];
        let push_constant_bytes = std::slice::from_raw_parts(
            push_constants.as_ptr() as *const u8,
            std::mem::size_of_val(&push_constants),
        );

        self.device.cmd_push_constants(
            command_buffer,
            self.composite_pipeline.pipeline_layout,
            vk::ShaderStageFlags::FRAGMENT,
            0,
            push_constant_bytes,
        );

        self.device.cmd_draw(command_buffer, 3, 1, 0, 0);

        Ok(())
    }
}
