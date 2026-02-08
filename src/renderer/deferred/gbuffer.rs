use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
use crate::asset::AssetStorage;
use crate::ecs::world::MeshRef;
use crate::ecs::World;
use crate::app::graphics_resource::{MeshBuffer, GraphicsResources};
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::core::{Device, RRDevice};
use crate::vulkanr::resource::{RRGBuffer, create_image, create_image_view};
use crate::vulkanr::render::RRRender;
use crate::vulkanr::render::pass::get_depth_format;

pub unsafe fn create_gbuffer_framebuffer(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &mut RRRender,
    gbuffer: &RRGBuffer,
) -> Result<()> {
    let (depth_image, depth_image_memory) = create_image(
        instance,
        rrdevice,
        gbuffer.width,
        gbuffer.height,
        1,
        vk::SampleCountFlags::_1,
        get_depth_format(instance, rrdevice)?,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    let depth_image_view = create_image_view(
        rrdevice,
        depth_image,
        get_depth_format(instance, rrdevice)?,
        vk::ImageAspectFlags::DEPTH,
        1,
    )?;

    rrrender.gbuffer_depth_image = depth_image;
    rrrender.gbuffer_depth_image_memory = depth_image_memory;
    rrrender.gbuffer_depth_image_view = depth_image_view;

    let attachments = [
        gbuffer.position_image_view,
        gbuffer.normal_image_view,
        gbuffer.albedo_image_view,
        gbuffer.object_id_image_view,
        depth_image_view,
    ];

    let info = vk::FramebufferCreateInfo::builder()
        .render_pass(rrrender.gbuffer_render_pass)
        .attachments(&attachments)
        .width(gbuffer.width)
        .height(gbuffer.height)
        .layers(1);

    rrrender.gbuffer_framebuffer = rrdevice.device.create_framebuffer(&info, None)?;

    log::info!("Created G-Buffer framebuffer: {}x{}", gbuffer.width, gbuffer.height);
    Ok(())
}

pub struct GBufferPass<'a> {
    gbuffer: &'a RRGBuffer,
    pipeline: &'a RRPipeline,
    graphics_resources: &'a GraphicsResources,
    meshes: &'a [MeshBuffer],
    device: &'a Device,
    ecs_world: &'a World,
    ecs_assets: &'a AssetStorage,
}

impl<'a> GBufferPass<'a> {
    pub fn new(app: &'a App) -> Result<Self> {
        let gbuffer = app.data.raytracing.gbuffer.as_ref()
            .ok_or_else(|| anyhow!("G-Buffer not initialized"))?;
        let pipeline = app.data.raytracing.gbuffer_pipeline.as_ref()
            .ok_or_else(|| anyhow!("G-Buffer pipeline not initialized"))?;

        Ok(Self {
            gbuffer,
            pipeline,
            graphics_resources: &app.data.graphics_resources,
            meshes: &app.data.graphics_resources.meshes,
            device: &app.rrdevice.device,
            ecs_world: &app.data.ecs_world,
            ecs_assets: &app.data.ecs_assets,
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

    fn create_clear_values(&self) -> [vk::ClearValue; 5] {
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
        let object_id_clear = vk::ClearValue {
            color: vk::ClearColorValue {
                uint32: [0, 0, 0, 0],
            },
        };
        let depth_clear = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 0.0,
                stencil: 0,
            },
        };

        [position_clear, normal_clear, albedo_clear, object_id_clear, depth_clear]
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
        if self.meshes.is_empty() {
            return Ok(());
        }

        let renderable_entities = self.ecs_world.query_renderable();
        let use_ecs = !renderable_entities.is_empty();

        if use_ecs {
            self.draw_meshes_ecs(command_buffer, image_index, &renderable_entities)?;
        } else {
            self.draw_meshes_legacy(command_buffer, image_index)?;
        }

        Ok(())
    }

    unsafe fn draw_meshes_ecs(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        entities: &[crate::ecs::Entity],
    ) -> Result<()> {
        for &entity in entities {
            let Some(mesh_ref) = self.ecs_world.get_component::<MeshRef>(entity) else {
                continue;
            };

            let Some(mesh_asset) = self.ecs_assets.get_mesh(mesh_ref.mesh_asset_id) else {
                continue;
            };

            let mesh_index = mesh_asset.graphics_mesh_index;
            if mesh_index >= self.meshes.len() {
                continue;
            }

            if !mesh_asset.render_to_gbuffer {
                continue;
            }

            let mesh = &self.meshes[mesh_index];
            self.draw_single_mesh(command_buffer, image_index, mesh, mesh_index)?;
        }

        Ok(())
    }

    unsafe fn draw_meshes_legacy(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        for i in 0..self.meshes.len() {
            let mesh = &self.meshes[i];

            if !mesh.render_to_gbuffer {
                continue;
            }

            self.draw_single_mesh(command_buffer, image_index, mesh, i)?;
        }

        Ok(())
    }

    unsafe fn draw_single_mesh(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        mesh: &MeshBuffer,
        mesh_index: usize,
    ) -> Result<()> {
        self.device.cmd_bind_vertex_buffers(
            command_buffer,
            0,
            &[mesh.vertex_buffer.buffer],
            &[0],
        );

        self.device.cmd_bind_index_buffer(
            command_buffer,
            mesh.index_buffer.buffer,
            0,
            vk::IndexType::UINT32,
        );

        let frame_set = self.graphics_resources.frame_set.sets[image_index];
        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline.pipeline_layout,
            0,
            &[frame_set],
            &[],
        );

        if let Some(material_id) = self.graphics_resources.get_material_id(mesh_index) {
            if let Some(material) = self.graphics_resources.materials.get(material_id) {
                self.device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline.pipeline_layout,
                    1,
                    &[material.descriptor_set],
                    &[],
                );
            }
        }

        let object_set_idx = self
            .graphics_resources
            .objects
            .get_set_index(image_index, mesh.object_index);
        let object_set = self.graphics_resources.objects.sets[object_set_idx];
        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline.pipeline_layout,
            2,
            &[object_set],
            &[],
        );

        let object_id: u32 = (mesh_index + 1) as u32;
        let object_id_bytes = object_id.to_ne_bytes();
        self.device.cmd_push_constants(
            command_buffer,
            self.pipeline.pipeline_layout,
            vk::ShaderStageFlags::FRAGMENT,
            0,
            &object_id_bytes,
        );

        self.device.cmd_draw_indexed(
            command_buffer,
            mesh.index_buffer.indices,
            1,
            0,
            0,
            0,
        );

        Ok(())
    }
}
