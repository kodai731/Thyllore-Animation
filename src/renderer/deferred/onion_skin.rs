use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::graphics_resource::GraphicsResources;
use crate::app::App;
use crate::renderer::deferred::gbuffer::OnionSkinPushConstants;
use crate::renderer::onion_skin_buffers::OnionSkinGpuState;
use crate::vulkanr::core::Device;
use crate::vulkanr::resource::OnionSkinPassResources;

pub struct OnionSkinRenderPass<'a> {
    resources: &'a OnionSkinPassResources,
    graphics_resources: &'a GraphicsResources,
    onion_skin_gpu: &'a OnionSkinGpuState,
    device: &'a Device,
}

impl<'a> OnionSkinRenderPass<'a> {
    pub fn new(app: &'a App) -> Result<Option<Self>> {
        let resources = match app.data.raytracing.onion_skin_pass {
            Some(ref r) => r,
            None => return Ok(None),
        };

        let onion_skin_gpu = match app.data.onion_skin_gpu {
            Some(ref gpu) => gpu,
            None => return Ok(None),
        };

        if onion_skin_gpu.source_mesh_index.is_none() {
            return Ok(None);
        }

        if onion_skin_gpu.active_ghost_count() == 0 {
            return Ok(None);
        }

        Ok(Some(Self {
            resources,
            graphics_resources: &app.data.graphics_resources,
            onion_skin_gpu,
            device: &app.rrdevice.device,
        }))
    }

    pub unsafe fn record(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        self.begin_render_pass(command_buffer);
        self.bind_pipeline_and_state(command_buffer);
        self.draw_ghosts(command_buffer, image_index)?;
        self.device.cmd_end_render_pass(command_buffer);
        Ok(())
    }

    unsafe fn begin_render_pass(&self, command_buffer: vk::CommandBuffer) {
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(vk::Extent2D {
                width: self.resources.width,
                height: self.resources.height,
            });

        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.resources.render_pass)
            .framebuffer(self.resources.framebuffer)
            .render_area(render_area)
            .clear_values(&[]);

        self.device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_info,
            vk::SubpassContents::INLINE,
        );
    }

    unsafe fn bind_pipeline_and_state(&self, command_buffer: vk::CommandBuffer) {
        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.resources.pipeline.pipeline,
        );

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(self.resources.width as f32)
            .height(self.resources.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(vk::Extent2D {
                width: self.resources.width,
                height: self.resources.height,
            });

        self.device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        self.device.cmd_set_scissor(command_buffer, 0, &[scissor]);
    }

    unsafe fn draw_ghosts(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        let source_mesh_index = self
            .onion_skin_gpu
            .source_mesh_index
            .ok_or_else(|| anyhow!("No source mesh index for onion skin"))?;

        if source_mesh_index >= self.graphics_resources.meshes.len() {
            return Ok(());
        }

        let source_mesh = &self.graphics_resources.meshes[source_mesh_index];
        let pipeline_layout = self.resources.pipeline.pipeline_layout;

        for ghost_buffer in &self.onion_skin_gpu.ghost_buffers {
            if ghost_buffer.vertex_count == 0 {
                continue;
            }

            self.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[ghost_buffer.vertex_buffer],
                &[0],
            );

            self.device.cmd_bind_index_buffer(
                command_buffer,
                self.onion_skin_gpu.source_index_buffer,
                0,
                vk::IndexType::UINT32,
            );

            let frame_set = self.graphics_resources.frame_set.sets[image_index];
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &[frame_set],
                &[],
            );

            if let Some(material_id) = self.graphics_resources.get_material_id(source_mesh_index) {
                if let Some(material) = self.graphics_resources.materials.get(material_id) {
                    self.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline_layout,
                        1,
                        &[material.descriptor_set],
                        &[],
                    );
                }
            }

            let object_set_idx = self
                .graphics_resources
                .objects
                .get_set_index(image_index, source_mesh.object_index);
            let object_set = self.graphics_resources.objects.sets[object_set_idx];
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                2,
                &[object_set],
                &[],
            );

            let push_constants =
                OnionSkinPushConstants::new(ghost_buffer.tint_color, ghost_buffer.opacity);
            self.device.cmd_push_constants(
                command_buffer,
                pipeline_layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                push_constants.as_bytes(),
            );

            self.device.cmd_draw_indexed(
                command_buffer,
                self.onion_skin_gpu.source_index_count,
                1,
                0,
                0,
                0,
            );
        }

        Ok(())
    }
}
