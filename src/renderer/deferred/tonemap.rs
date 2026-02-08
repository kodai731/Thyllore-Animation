use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
use crate::debugview::gizmo::{BoneDisplayStyle, BoneGizmoData, ConstraintGizmoData};
use crate::app::graphics_resource::GraphicsResources;
use crate::ecs::component::LineMesh;
use crate::ecs::resource::{BloomSettings, LensEffects, ToneMapping};
use crate::vulkanr::core::Device;
use crate::vulkanr::descriptor::RRToneMapDescriptorSet;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::resource::{GpuBufferRegistry, PipelineStorage};

#[repr(C)]
#[derive(Clone, Copy)]
struct ToneMapPushConstants {
    tone_map_operator: i32,
    gamma: f32,
    exposure_value: f32,
    vignette_intensity: f32,
    chromatic_aberration_intensity: f32,
    bloom_intensity: f32,
}

pub struct ToneMapPass<'a> {
    app: &'a App,
    tonemap_pipeline: &'a RRPipeline,
    tonemap_descriptor: &'a RRToneMapDescriptorSet,
    graphics_resources: &'a GraphicsResources,
    buffer_registry: &'a GpuBufferRegistry,
    device: &'a Device,
    extent: vk::Extent2D,
}

impl<'a> ToneMapPass<'a> {
    pub fn new(app: &'a App, extent: vk::Extent2D) -> Result<Self> {
        let tonemap_pipeline = app
            .data
            .raytracing
            .tonemap_pipeline
            .as_ref()
            .ok_or_else(|| anyhow!("ToneMap pipeline not initialized"))?;
        let tonemap_descriptor = app
            .data
            .raytracing
            .tonemap_descriptor
            .as_ref()
            .ok_or_else(|| anyhow!("ToneMap descriptor not initialized"))?;

        Ok(Self {
            app,
            tonemap_pipeline,
            tonemap_descriptor,
            graphics_resources: &app.data.graphics_resources,
            buffer_registry: &app.data.buffer_registry,
            device: &app.rrdevice.device,
            extent,
        })
    }

    fn pipeline_storage(&self) -> &PipelineStorage {
        self.app.pipeline_storage()
    }

    pub unsafe fn record_to_offscreen(
        &self,
        command_buffer: vk::CommandBuffer,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        image_index: usize,
    ) -> Result<()> {
        self.begin_render_pass(command_buffer, render_pass, framebuffer);
        self.draw_tonemap(command_buffer)?;
        self.draw_grid(command_buffer, image_index)?;
        self.draw_gizmo(command_buffer, image_index)?;

        let grid_mesh = self.app.grid_mesh();
        let light_gizmo = self.app.light_gizmo();
        let pipeline_storage = self.pipeline_storage();

        if let Some(pipeline_id) = grid_mesh.render_info.pipeline_id {
            if let Some(pipeline) = pipeline_storage.get(pipeline_id) {
                self.draw_line_mesh(
                    &light_gizmo.ray_to_model,
                    pipeline,
                    grid_mesh.render_info.object_index,
                    command_buffer,
                    image_index,
                );
                self.draw_line_mesh(
                    &light_gizmo.vertical_lines,
                    pipeline,
                    grid_mesh.render_info.object_index,
                    command_buffer,
                    image_index,
                );
            }
        }

        self.draw_bone_gizmo(command_buffer, image_index);
        self.draw_constraint_gizmo(command_buffer, image_index);
        self.draw_billboard(command_buffer, image_index)?;
        self.device.cmd_end_render_pass(command_buffer);

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
            .extent(self.extent);

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
        let resolve_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let clear_values = vec![color_clear_value, depth_clear_value, resolve_clear_value];

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

    unsafe fn draw_tonemap(&self, command_buffer: vk::CommandBuffer) -> Result<()> {
        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.tonemap_pipeline.pipeline,
        );

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(self.extent.width as f32)
            .height(self.extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(self.extent);

        self.device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        self.device.cmd_set_scissor(command_buffer, 0, &[scissor]);

        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.tonemap_pipeline.pipeline_layout,
            0,
            &[self.tonemap_descriptor.descriptor_set],
            &[],
        );

        let (operator, gamma) = match self.app.data.ecs_world.get_resource::<ToneMapping>() {
            Some(tm) => {
                let op = if tm.enabled { tm.operator as i32 } else { 0 };
                (op, tm.gamma)
            }
            None => (0, 2.2),
        };

        let exposure_value = self
            .app
            .data
            .ecs_world
            .get_resource::<crate::ecs::resource::Exposure>()
            .map(|e| e.exposure_value)
            .unwrap_or(1.0);

        let (vignette_intensity, ca_intensity) =
            match self.app.data.ecs_world.get_resource::<LensEffects>() {
                Some(le) => {
                    let vi = if le.vignette_enabled {
                        le.vignette_intensity
                    } else {
                        0.0
                    };
                    let ca = if le.chromatic_aberration_enabled {
                        le.chromatic_aberration_intensity
                    } else {
                        0.0
                    };
                    (vi, ca)
                }
                None => (0.0, 0.0),
            };

        let bloom_intensity = self
            .app
            .data
            .ecs_world
            .get_resource::<BloomSettings>()
            .map(|bs| if bs.enabled { bs.intensity } else { 0.0 })
            .unwrap_or(0.0);

        let push_constants = ToneMapPushConstants {
            tone_map_operator: operator,
            gamma,
            exposure_value,
            vignette_intensity,
            chromatic_aberration_intensity: ca_intensity,
            bloom_intensity,
        };

        let push_constant_bytes = std::slice::from_raw_parts(
            &push_constants as *const ToneMapPushConstants as *const u8,
            std::mem::size_of::<ToneMapPushConstants>(),
        );

        self.device.cmd_push_constants(
            command_buffer,
            self.tonemap_pipeline.pipeline_layout,
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
        let grid = self.app.grid_mesh();
        let pipeline_storage = self.pipeline_storage();

        let vertex_buffer = match self
            .buffer_registry
            .get_vertex_buffer(grid.mesh.vertex_buffer_handle)
        {
            Some(b) => b,
            None => return Ok(()),
        };
        let index_buffer = match self
            .buffer_registry
            .get_index_buffer(grid.mesh.index_buffer_handle)
        {
            Some(b) => b,
            None => return Ok(()),
        };

        let pipeline_id = match grid.render_info.pipeline_id {
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
            .get_set_index(image_index, grid.render_info.object_index);
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
            .cmd_draw_indexed(command_buffer, grid.mesh.indices.len() as u32, 1, 0, 0, 0);

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

        let pipeline_id = match gizmo.render_info.pipeline_id {
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
            .get_set_index(image_index, gizmo.render_info.object_index);
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

    unsafe fn draw_line_mesh(
        &self,
        mesh: &LineMesh,
        pipeline: &RRPipeline,
        object_index: usize,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) {
        if mesh.indices.is_empty() {
            return;
        }

        let vertex_buffer = match self
            .buffer_registry
            .get_vertex_buffer(mesh.vertex_buffer_handle)
        {
            Some(vb) => vb,
            None => return,
        };
        let index_buffer = match self
            .buffer_registry
            .get_index_buffer(mesh.index_buffer_handle)
        {
            Some(ib) => ib,
            None => return,
        };

        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline,
        );

        self.device.cmd_set_line_width(command_buffer, 1.0);
        self.device
            .cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
        self.device
            .cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);

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
            .get_set_index(image_index, object_index);
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
            .cmd_draw_indexed(command_buffer, mesh.indices.len() as u32, 1, 0, 0, 0);
    }

    unsafe fn push_bone_alpha(
        &self,
        command_buffer: vk::CommandBuffer,
        pipeline: &RRPipeline,
        alpha: f32,
    ) {
        let alpha_bytes = std::slice::from_raw_parts(
            &alpha as *const f32 as *const u8,
            std::mem::size_of::<f32>(),
        );
        self.device.cmd_push_constants(
            command_buffer,
            pipeline.pipeline_layout,
            vk::ShaderStageFlags::FRAGMENT,
            0,
            alpha_bytes,
        );
    }

    unsafe fn draw_bone_solid_pass(
        &self,
        mesh: &LineMesh,
        render_info: &crate::ecs::component::RenderInfo,
        alpha: f32,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) {
        if mesh.indices.is_empty() {
            return;
        }
        let Some(pid) = render_info.pipeline_id else {
            return;
        };
        let Some(pipeline) = self.pipeline_storage().get(pid) else {
            return;
        };
        self.push_bone_alpha(command_buffer, pipeline, alpha);
        self.draw_triangle_mesh(
            mesh,
            pipeline,
            render_info.object_index,
            command_buffer,
            image_index,
        );
    }

    unsafe fn draw_bone_wire_pass(
        &self,
        mesh: &LineMesh,
        render_info: &crate::ecs::component::RenderInfo,
        alpha: f32,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) {
        if mesh.indices.is_empty() {
            return;
        }
        let Some(pid) = render_info.pipeline_id else {
            return;
        };
        let Some(pipeline) = self.pipeline_storage().get(pid) else {
            return;
        };
        self.push_bone_alpha(command_buffer, pipeline, alpha);
        self.draw_line_mesh(
            mesh,
            pipeline,
            render_info.object_index,
            command_buffer,
            image_index,
        );
    }

    unsafe fn draw_bone_gizmo(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) {
        let bone_gizmo = match self.app.get_resource::<BoneGizmoData>() {
            Some(bg) => bg,
            None => return,
        };

        if !bone_gizmo.visible {
            return;
        }

        match bone_gizmo.display_style {
            BoneDisplayStyle::Stick => {
                if bone_gizmo.stick_mesh.indices.is_empty() {
                    return;
                }
                let pipeline_id = match bone_gizmo.stick_render_info.pipeline_id {
                    Some(id) => id,
                    None => return,
                };
                let pipeline = match self.pipeline_storage().get(pipeline_id) {
                    Some(p) => p,
                    None => return,
                };
                self.draw_line_mesh(
                    &bone_gizmo.stick_mesh,
                    pipeline,
                    bone_gizmo.stick_render_info.object_index,
                    command_buffer,
                    image_index,
                );
            }

            BoneDisplayStyle::Octahedral
            | BoneDisplayStyle::Box
            | BoneDisplayStyle::Sphere => {
                if bone_gizmo.in_front {
                    self.draw_bone_solid_pass(
                        &bone_gizmo.solid_mesh,
                        &bone_gizmo.solid_render_info,
                        1.0,
                        command_buffer,
                        image_index,
                    );
                    self.draw_bone_wire_pass(
                        &bone_gizmo.wire_mesh,
                        &bone_gizmo.wire_render_info,
                        1.0,
                        command_buffer,
                        image_index,
                    );
                } else {
                    self.draw_bone_solid_pass(
                        &bone_gizmo.solid_mesh,
                        &bone_gizmo.solid_depth_render_info,
                        1.0,
                        command_buffer,
                        image_index,
                    );
                    self.draw_bone_wire_pass(
                        &bone_gizmo.wire_mesh,
                        &bone_gizmo.wire_depth_render_info,
                        1.0,
                        command_buffer,
                        image_index,
                    );

                    self.draw_bone_solid_pass(
                        &bone_gizmo.solid_mesh,
                        &bone_gizmo.solid_occluded_render_info,
                        0.25,
                        command_buffer,
                        image_index,
                    );
                    self.draw_bone_wire_pass(
                        &bone_gizmo.wire_mesh,
                        &bone_gizmo.wire_occluded_render_info,
                        0.25,
                        command_buffer,
                        image_index,
                    );
                }
            }
        }
    }

    unsafe fn draw_constraint_gizmo(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) {
        let constraint_gizmo =
            match self.app.get_resource::<ConstraintGizmoData>() {
                Some(cg) => cg,
                None => return,
            };

        if !constraint_gizmo.visible {
            return;
        }
        if constraint_gizmo.wire_mesh.indices.is_empty() {
            return;
        }

        let pipeline_id =
            match constraint_gizmo.wire_render_info.pipeline_id {
                Some(id) => id,
                None => return,
            };
        let pipeline = match self.pipeline_storage().get(pipeline_id) {
            Some(p) => p,
            None => return,
        };

        self.draw_line_mesh(
            &constraint_gizmo.wire_mesh,
            pipeline,
            constraint_gizmo.wire_render_info.object_index,
            command_buffer,
            image_index,
        );
    }

    unsafe fn draw_triangle_mesh(
        &self,
        mesh: &LineMesh,
        pipeline: &RRPipeline,
        object_index: usize,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) {
        if mesh.indices.is_empty() {
            return;
        }

        let vertex_buffer = match self
            .buffer_registry
            .get_vertex_buffer(mesh.vertex_buffer_handle)
        {
            Some(vb) => vb,
            None => return,
        };
        let index_buffer = match self
            .buffer_registry
            .get_index_buffer(mesh.index_buffer_handle)
        {
            Some(ib) => ib,
            None => return,
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
            .get_set_index(image_index, object_index);
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
            .cmd_draw_indexed(command_buffer, mesh.indices.len() as u32, 1, 0, 0, 0);
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
            .get_vertex_buffer(billboard.mesh.vertex_buffer_handle)
        {
            Some(b) => b,
            None => return Ok(()),
        };
        let index_buffer = match self
            .buffer_registry
            .get_index_buffer(billboard.mesh.index_buffer_handle)
        {
            Some(b) => b,
            None => return Ok(()),
        };

        let pipeline_id = match billboard.render_info.pipeline_id {
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
            &[billboard.render_state.descriptor_set.descriptor_sets[descriptor_set_index]],
            &[],
        );

        self.device.cmd_draw_indexed(
            command_buffer,
            billboard.mesh.indices.len() as u32,
            1,
            0,
            0,
            0,
        );

        Ok(())
    }
}
