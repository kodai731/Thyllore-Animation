use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
use crate::debugview::DebugViewMode;
use crate::ecs::systems::{gizmo_draw_ray, gizmo_draw_vertical_lines};
use crate::scene::graphics_resource::GraphicsResources;
use crate::scene::Scene;
use crate::vulkanr::core::Device;
use crate::vulkanr::descriptor::RRCompositeDescriptorSet;
use crate::vulkanr::pipeline::RRPipeline;

pub struct CompositePass<'a> {
    composite_pipeline: &'a RRPipeline,
    composite_descriptor: &'a RRCompositeDescriptorSet,
    scene: &'a Scene,
    graphics_resources: &'a GraphicsResources,
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
            composite_pipeline,
            composite_descriptor,
            scene: &app.scene,
            graphics_resources: &app.data.graphics_resources,
            device: &app.rrdevice.device,
            swapchain_extent: app.swapchain_state().swapchain.swapchain_extent,
            debug_view_mode: app.rt_debug_state().debug_view_mode,
        })
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

        let grid = self.scene.grid();
        let light_gizmo = self.scene.light_gizmo();

        gizmo_draw_ray(
            &light_gizmo.ray_to_model,
            self.device,
            command_buffer,
            &grid.pipeline,
            self.graphics_resources,
            grid.object_index,
            image_index,
        );
        gizmo_draw_vertical_lines(
            &light_gizmo.vertical_lines,
            self.device,
            command_buffer,
            &grid.pipeline,
            self.graphics_resources,
            grid.object_index,
            image_index,
        );
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
        let grid = self.scene.grid();

        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            grid.pipeline.pipeline,
        );

        self.device.cmd_set_line_width(command_buffer, 1.0);

        self.device
            .cmd_bind_vertex_buffers(command_buffer, 0, &[grid.vertex_buffer.buffer], &[0]);

        self.device.cmd_bind_index_buffer(
            command_buffer,
            grid.index_buffer.buffer,
            0,
            vk::IndexType::UINT32,
        );

        let frame_set = self.graphics_resources.frame_set.sets[image_index];
        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            grid.pipeline.pipeline_layout,
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
            grid.pipeline.pipeline_layout,
            2,
            &[object_set],
            &[],
        );

        self.device
            .cmd_draw_indexed(command_buffer, grid.index_buffer.indices, 1, 0, 0, 0);

        Ok(())
    }

    unsafe fn draw_gizmo(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        let gizmo = self.scene.gizmo();

        if let (Some(vertex_buffer), Some(index_buffer)) =
            (gizmo.mesh.vertex_buffer, gizmo.mesh.index_buffer)
        {
            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                gizmo.mesh.pipeline.pipeline,
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
                gizmo.mesh.pipeline.pipeline_layout,
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
                gizmo.mesh.pipeline.pipeline_layout,
                2,
                &[object_set],
                &[],
            );

            self.device
                .cmd_draw_indexed(command_buffer, gizmo.mesh.indices.len() as u32, 1, 0, 0, 0);
        }

        Ok(())
    }

    unsafe fn draw_billboard(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        let billboard = self.scene.billboard();

        if let (Some(vertex_buffer), Some(index_buffer)) =
            (billboard.vertex_buffer, billboard.index_buffer)
        {
            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                billboard.pipeline.pipeline,
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
                billboard.pipeline.pipeline_layout,
                0,
                &[billboard.descriptor_set.descriptor_sets[descriptor_set_index]],
                &[],
            );

            self.device.cmd_draw_indexed(
                command_buffer,
                billboard.indices.len() as u32,
                1,
                0,
                0,
                0,
            );
        }

        Ok(())
    }
}
