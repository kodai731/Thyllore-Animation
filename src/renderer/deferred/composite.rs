use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use super::gizmo::{gizmo_draw_ray_with_pipeline, gizmo_draw_vertical_lines_with_pipeline};
use crate::app::App;
use crate::debugview::DebugViewMode;
use crate::scene::graphics_resource::GraphicsResources;
use crate::vulkanr::core::Device;
use crate::vulkanr::descriptor::RRCompositeDescriptorSet;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::resource::{GpuBufferRegistry, PipelineStorage};

pub struct CompositePass<'a> {
    app: &'a App,
    composite_pipeline: &'a RRPipeline,
    composite_descriptor: &'a RRCompositeDescriptorSet,
    graphics_resources: &'a GraphicsResources,
    buffer_registry: &'a GpuBufferRegistry,
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
            graphics_resources: &app.data.graphics_resources,
            buffer_registry: &app.data.buffer_registry,
            device: &app.rrdevice.device,
            swapchain_extent: app.swapchain_state().swapchain.swapchain_extent,
            debug_view_mode: app.rt_debug_state().debug_view_mode,
        })
    }

    fn pipeline_storage(&self) -> &PipelineStorage {
        self.app.pipeline_storage()
    }

    pub unsafe fn record(
        &self,
        command_buffer: vk::CommandBuffer,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        image_index: usize,
    ) -> Result<()> {
        self.begin_render_pass(command_buffer, render_pass, framebuffer);
        self.draw_composite(command_buffer)?;
        self.draw_grid(command_buffer, image_index)?;
        self.draw_gizmo(command_buffer, image_index)?;

        let grid = self.app.grid();
        let light_gizmo = self.app.light_gizmo();
        let pipeline_storage = self.pipeline_storage();

        if let Some(pipeline_id) = grid.pipeline_id {
            if let Some(pipeline) = pipeline_storage.get(pipeline_id) {
                gizmo_draw_ray_with_pipeline(
                    &light_gizmo.ray_to_model,
                    self.buffer_registry,
                    self.device,
                    command_buffer,
                    pipeline,
                    self.graphics_resources,
                    grid.object_index,
                    image_index,
                );
                gizmo_draw_vertical_lines_with_pipeline(
                    &light_gizmo.vertical_lines,
                    self.buffer_registry,
                    self.device,
                    command_buffer,
                    pipeline,
                    self.graphics_resources,
                    grid.object_index,
                    image_index,
                );
            }
        }

        self.draw_billboard(command_buffer, image_index)?;

        Ok(())
    }

    unsafe fn begin_render_pass(
        &self,
        command_buffer: vk::CommandBuffer,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
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
                depth: 1.0,
                stencil: 0,
            },
        };
        let clear_values = [color_clear_value, depth_clear_value];

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

    unsafe fn draw_grid(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        let grid = self.app.grid();
        let pipeline_storage = self.pipeline_storage();

        let vertex_buffer = match self
            .buffer_registry
            .get_vertex_buffer(grid.vertex_buffer_handle)
        {
            Some(b) => b,
            None => return Ok(()),
        };
        let index_buffer = match self
            .buffer_registry
            .get_index_buffer(grid.index_buffer_handle)
        {
            Some(b) => b,
            None => return Ok(()),
        };

        let pipeline_id = match grid.pipeline_id {
            Some(id) => id,
            None => return Ok(()),
        };
        let pipeline = match pipeline_storage.get(pipeline_id) {
            Some(p) => p,
            None => return Ok(()),
        };

        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline,
        );

        self.device.cmd_set_line_width(command_buffer, 1.0);

        self.device
            .cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);

        self.device.cmd_bind_index_buffer(
            command_buffer,
            index_buffer,
            0,
            vk::IndexType::UINT32,
        );

        let frame_set = self.graphics_resources.frame_set.sets[image_index];
        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline_layout,
            0,
            &[frame_set],
            &[],
        );

        let object_set_idx = self
            .graphics_resources
            .objects
            .get_set_index(image_index, grid.object_index);
        let object_set = self.graphics_resources.objects.sets[object_set_idx];
        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline_layout,
            2,
            &[object_set],
            &[],
        );

        self.device
            .cmd_draw_indexed(command_buffer, grid.indices.len() as u32, 1, 0, 0, 0);

        Ok(())
    }

    unsafe fn draw_gizmo(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        let gizmo = self.app.grid_gizmo();
        let pipeline_storage = self.pipeline_storage();

        let vertex_buffer = match self
            .buffer_registry
            .get_vertex_buffer(gizmo.mesh.vertex_buffer_handle)
        {
            Some(b) => b,
            None => return Ok(()),
        };
        let index_buffer = match self
            .buffer_registry
            .get_index_buffer(gizmo.mesh.index_buffer_handle)
        {
            Some(b) => b,
            None => return Ok(()),
        };

        let pipeline_id = match gizmo.mesh.pipeline_id {
            Some(id) => id,
            None => return Ok(()),
        };
        let pipeline = match pipeline_storage.get(pipeline_id) {
            Some(p) => p,
            None => return Ok(()),
        };

        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline,
        );

        self.device.cmd_set_line_width(command_buffer, 1.0);

        self.device
            .cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);

        self.device.cmd_bind_index_buffer(
            command_buffer,
            index_buffer,
            0,
            vk::IndexType::UINT32,
        );

        let frame_set = self.graphics_resources.frame_set.sets[image_index];
        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline_layout,
            0,
            &[frame_set],
            &[],
        );

        let object_set_idx = self
            .graphics_resources
            .objects
            .get_set_index(image_index, gizmo.mesh.object_index);
        let object_set = self.graphics_resources.objects.sets[object_set_idx];
        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline_layout,
            2,
            &[object_set],
            &[],
        );

        self.device
            .cmd_draw_indexed(command_buffer, gizmo.mesh.indices.len() as u32, 1, 0, 0, 0);

        Ok(())
    }

    unsafe fn draw_billboard(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        let billboard = self.app.billboard();
        let pipeline_storage = self.pipeline_storage();

        let vertex_buffer = match self
            .buffer_registry
            .get_vertex_buffer(billboard.info.vertex_buffer_handle)
        {
            Some(b) => b,
            None => return Ok(()),
        };
        let index_buffer = match self
            .buffer_registry
            .get_index_buffer(billboard.info.index_buffer_handle)
        {
            Some(b) => b,
            None => return Ok(()),
        };

        let pipeline_id = match billboard.render.pipeline_id {
            Some(id) => id,
            None => return Ok(()),
        };
        let pipeline = match pipeline_storage.get(pipeline_id) {
            Some(p) => p,
            None => return Ok(()),
        };

        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline,
        );

        self.device
            .cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);

        self.device.cmd_bind_index_buffer(
            command_buffer,
            index_buffer,
            0,
            vk::IndexType::UINT32,
        );

        let descriptor_set_index = image_index;

        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline_layout,
            0,
            &[billboard.render.descriptor_set.descriptor_sets[descriptor_set_index]],
            &[],
        );

        self.device.cmd_draw_indexed(
            command_buffer,
            billboard.info.indices.len() as u32,
            1,
            0,
            0,
            0,
        );

        Ok(())
    }
}
