use std::ffi::c_void;
use std::mem::size_of;
use std::rc::Rc;

use anyhow::Result;
use cgmath::{Matrix4, Vector3, Vector4};
use vulkanalia::prelude::v1_0::*;

use crate::ecs::component::{GizmoMesh, GizmoRayToModel, GizmoVerticalLines};
use crate::ecs::systems::ProjectionData;
use crate::render::{FrameUBO, IndexBufferHandle, MeshId, ObjectUBO, RenderBackend, VertexBufferHandle};
use crate::scene::billboard::BillboardData;
use crate::scene::graphics_resource::GraphicsResources;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::core::device::RRDevice;
use crate::vulkanr::data::Vertex;
use crate::vulkanr::image::RRImage;
use crate::vulkanr::raytracing::acceleration::RRAccelerationStructure;
use crate::vulkanr::resource::GpuBufferRegistry;
use crate::vulkanr::vulkan::Instance;

pub struct VulkanBackend<'a> {
    pub instance: &'a Instance,
    pub device: &'a RRDevice,
    pub command_pool: Rc<RRCommandPool>,
    pub graphics: &'a mut GraphicsResources,
    pub acceleration_structure: &'a mut Option<RRAccelerationStructure>,
    pub buffer_registry: &'a mut GpuBufferRegistry,
}

impl<'a> VulkanBackend<'a> {
    pub fn new(
        instance: &'a Instance,
        device: &'a RRDevice,
        command_pool: Rc<RRCommandPool>,
        graphics: &'a mut GraphicsResources,
        acceleration_structure: &'a mut Option<RRAccelerationStructure>,
        buffer_registry: &'a mut GpuBufferRegistry,
    ) -> Self {
        Self {
            instance,
            device,
            command_pool,
            graphics,
            acceleration_structure,
            buffer_registry,
        }
    }
}

impl<'a> RenderBackend for VulkanBackend<'a> {
    unsafe fn upload_mesh_vertices(&mut self, mesh_id: MeshId) -> Result<()> {
        if mesh_id >= self.graphics.meshes.len() {
            return Ok(());
        }

        let mesh = &mut self.graphics.meshes[mesh_id];
        let vertices = &mesh.vertex_data.vertices;
        let vertex_count = vertices.len();
        let vertex_stride = size_of::<Vertex>();

        mesh.vertex_buffer.update(
            self.instance,
            self.device,
            self.command_pool.as_ref(),
            (vertex_stride * vertex_count) as vk::DeviceSize,
            vertices.as_ptr() as *const c_void,
            vertex_count,
        )?;

        Ok(())
    }

    unsafe fn update_acceleration_structure(&mut self, mesh_ids: &[MeshId]) -> Result<()> {
        let Some(ref mut accel_struct) = self.acceleration_structure else {
            return Ok(());
        };

        for &mesh_id in mesh_ids {
            if mesh_id >= self.graphics.meshes.len() {
                continue;
            }
            if mesh_id >= accel_struct.blas_list.len() {
                continue;
            }

            let mesh = &self.graphics.meshes[mesh_id];
            let blas = &accel_struct.blas_list[mesh_id];

            RRAccelerationStructure::update_blas(
                self.instance,
                self.device,
                self.command_pool.as_ref(),
                blas,
                &mesh.vertex_buffer.buffer,
                mesh.vertex_data.vertices.len() as u32,
                size_of::<Vertex>() as u32,
                &mesh.index_buffer.buffer,
                mesh.vertex_data.indices.len() as u32,
            )?;
        }

        Ok(())
    }

    unsafe fn rebuild_tlas(&mut self) -> Result<()> {
        let Some(ref mut accel_struct) = self.acceleration_structure else {
            return Ok(());
        };

        let tlas = &accel_struct.tlas;
        RRAccelerationStructure::update_tlas(
            self.instance,
            self.device,
            self.command_pool.as_ref(),
            tlas,
            &accel_struct.blas_list,
        )?;

        Ok(())
    }

    unsafe fn create_gizmo_buffers(
        &mut self,
        mesh: &mut GizmoMesh,
        use_staging: bool,
    ) -> Result<()> {
        let vertex_handle = if use_staging {
            self.buffer_registry.create_vertex_buffer(
                self.instance,
                self.device,
                self.command_pool.as_ref(),
                &mesh.vertices,
                true,
            )?
        } else {
            self.buffer_registry.create_host_visible_vertex_buffer(
                self.instance,
                self.device,
                &mesh.vertices,
                0,
            )?
        };
        mesh.vertex_buffer_handle = vertex_handle;

        let index_handle = self.buffer_registry.create_index_buffer(
            self.instance,
            self.device,
            self.command_pool.as_ref(),
            &mesh.indices,
        )?;
        mesh.index_buffer_handle = index_handle;

        Ok(())
    }

    unsafe fn update_gizmo_vertex_buffer(&self, mesh: &GizmoMesh) -> Result<()> {
        self.buffer_registry
            .update_vertex_buffer(self.device, mesh.vertex_buffer_handle, &mesh.vertices)?;
        Ok(())
    }

    unsafe fn destroy_gizmo_buffers(&mut self, mesh: &mut GizmoMesh) {
        self.buffer_registry
            .destroy_vertex_buffer(self.device, mesh.vertex_buffer_handle);
        self.buffer_registry
            .destroy_index_buffer(self.device, mesh.index_buffer_handle);
        mesh.vertex_buffer_handle = VertexBufferHandle::INVALID;
        mesh.index_buffer_handle = IndexBufferHandle::INVALID;
    }

    unsafe fn update_or_create_ray_buffers(&mut self, ray: &mut GizmoRayToModel) -> Result<()> {
        if ray.vertices.is_empty() {
            return Ok(());
        }

        if !ray.vertex_buffer_handle.is_valid() {
            let vertex_handle = self.buffer_registry.create_host_visible_vertex_buffer(
                self.instance,
                self.device,
                &ray.vertices,
                0,
            )?;
            ray.vertex_buffer_handle = vertex_handle;
        } else {
            self.buffer_registry
                .update_vertex_buffer(self.device, ray.vertex_buffer_handle, &ray.vertices)?;
        }

        if !ray.index_buffer_handle.is_valid() {
            let index_handle = self.buffer_registry.create_host_visible_index_buffer(
                self.instance,
                self.device,
                &ray.indices,
            )?;
            ray.index_buffer_handle = index_handle;
        } else {
            self.buffer_registry
                .update_index_buffer(self.device, ray.index_buffer_handle, &ray.indices)?;
        }

        Ok(())
    }

    unsafe fn destroy_ray_buffers(&mut self, ray: &mut GizmoRayToModel) {
        self.buffer_registry
            .destroy_vertex_buffer(self.device, ray.vertex_buffer_handle);
        self.buffer_registry
            .destroy_index_buffer(self.device, ray.index_buffer_handle);
        ray.vertex_buffer_handle = VertexBufferHandle::INVALID;
        ray.index_buffer_handle = IndexBufferHandle::INVALID;
    }

    unsafe fn update_or_create_vertical_line_buffers(
        &mut self,
        lines: &mut GizmoVerticalLines,
    ) -> Result<()> {
        if lines.vertices.is_empty() {
            return Ok(());
        }

        if !lines.vertex_buffer_handle.is_valid() {
            let vertex_handle = self.buffer_registry.create_host_visible_vertex_buffer(
                self.instance,
                self.device,
                &lines.vertices,
                1024,
            )?;
            lines.vertex_buffer_handle = vertex_handle;
        } else {
            self.buffer_registry.update_vertex_buffer(
                self.device,
                lines.vertex_buffer_handle,
                &lines.vertices,
            )?;
        }

        if !lines.index_buffer_handle.is_valid() {
            let index_handle = self.buffer_registry.create_host_visible_index_buffer(
                self.instance,
                self.device,
                &lines.indices,
            )?;
            lines.index_buffer_handle = index_handle;
        } else {
            self.buffer_registry
                .update_index_buffer(self.device, lines.index_buffer_handle, &lines.indices)?;
        }

        Ok(())
    }

    unsafe fn destroy_vertical_line_buffers(&mut self, lines: &mut GizmoVerticalLines) {
        self.buffer_registry
            .destroy_vertex_buffer(self.device, lines.vertex_buffer_handle);
        self.buffer_registry
            .destroy_index_buffer(self.device, lines.index_buffer_handle);
        lines.vertex_buffer_handle = VertexBufferHandle::INVALID;
        lines.index_buffer_handle = IndexBufferHandle::INVALID;
    }

    unsafe fn create_billboard_buffers(&mut self, billboard: &mut BillboardData) -> Result<()> {
        billboard.vertex_buffer_handle = self.buffer_registry.create_host_visible_vertex_buffer(
            self.instance,
            self.device,
            &billboard.vertices,
            256,
        )?;

        billboard.index_buffer_handle = self.buffer_registry.create_host_visible_index_buffer(
            self.instance,
            self.device,
            &billboard.indices,
        )?;

        let texture_path = std::path::Path::new("assets/textures/lightIcon.png");
        billboard.texture = Some(
            RRImage::new_from_file(
                self.instance,
                self.device,
                self.command_pool.as_ref(),
                texture_path,
            )
            .map_err(|e| anyhow::anyhow!("Failed to load billboard texture: {}", e))?,
        );

        Ok(())
    }

    unsafe fn update_frame_ubo(
        &mut self,
        proj_data: &ProjectionData,
        camera_pos: Vector3<f32>,
        light_pos: Vector3<f32>,
        light_color: Vector3<f32>,
        image_index: usize,
    ) -> Result<()> {
        let ubo = FrameUBO {
            view: proj_data.view,
            proj: proj_data.proj,
            camera_pos: Vector4::new(camera_pos.x, camera_pos.y, camera_pos.z, 1.0),
            light_pos: Vector4::new(light_pos.x, light_pos.y, light_pos.z, 1.0),
            light_color: Vector4::new(light_color.x, light_color.y, light_color.z, 1.0),
        };

        self.graphics
            .frame_set
            .update(self.device, image_index, &ubo)?;

        Ok(())
    }

    unsafe fn update_object_ubo(
        &mut self,
        model_matrix: Matrix4<f32>,
        object_index: usize,
        image_index: usize,
    ) -> Result<()> {
        let ubo = ObjectUBO { model: model_matrix };
        self.graphics
            .objects
            .update(self.device, image_index, object_index, &ubo)?;
        Ok(())
    }
}
