use std::mem::size_of;
use std::rc::Rc;

use anyhow::Result;
use cgmath::{Matrix4, Vector3, Vector4};

use thyllore_render_core::{
    BillboardMesh, BufferMemoryType, DistanceAttenuation, FrameUBO, IndexBufferHandle, LineMesh,
    MeshId, ObjectUBO, ProjectionData, StaticLineMesh, VertexBufferHandle, FRAMES_IN_FLIGHT,
};

use crate::command::RRCommandPool;
use crate::core::device::RRDevice;
use crate::data::{SceneUniformData, UniformBufferObject, Vertex};
use crate::raytracing::RRAccelerationStructure;
use crate::resource::graphics_resource::GraphicsResources;
use crate::resource::image::RRImage;
use crate::resource::raytracing_data::RayTracingData;
use crate::resource::{BillboardRenderState, GpuBufferRegistry};
use crate::types::Instance;

pub struct VulkanBackend<'a> {
    pub instance: &'a Instance,
    pub device: &'a RRDevice,
    pub command_pool: Rc<RRCommandPool>,
    pub graphics: &'a mut GraphicsResources,
    pub raytracing: &'a mut RayTracingData,
    pub buffer_registry: &'a mut GpuBufferRegistry,
}

impl<'a> VulkanBackend<'a> {
    pub fn new(
        instance: &'a Instance,
        device: &'a RRDevice,
        command_pool: Rc<RRCommandPool>,
        graphics: &'a mut GraphicsResources,
        raytracing: &'a mut RayTracingData,
        buffer_registry: &'a mut GpuBufferRegistry,
    ) -> Self {
        Self {
            instance,
            device,
            command_pool,
            graphics,
            raytracing,
            buffer_registry,
        }
    }

    pub unsafe fn create_billboard_buffers(
        &mut self,
        mesh: &mut BillboardMesh,
        render_state: &mut BillboardRenderState,
    ) -> Result<()> {
        mesh.vertex_buffer_handle = self.buffer_registry.create_host_visible_vertex_buffer(
            self.instance,
            self.device,
            &mesh.vertices,
            256,
        )?;

        mesh.index_buffer_handle = self.buffer_registry.create_host_visible_index_buffer(
            self.instance,
            self.device,
            &mesh.indices,
        )?;

        let white_pixel: [u8; 4] = [255, 255, 255, 255];
        render_state.texture = Some(
            RRImage::new_from_pixels(
                self.instance,
                self.device,
                &self.command_pool,
                &white_pixel,
                1,
                1,
            )
            .map_err(|e| anyhow::anyhow!("Failed to create billboard texture: {}", e))?,
        );

        Ok(())
    }

    pub unsafe fn update_billboard_ubo(
        &mut self,
        render_state: &mut BillboardRenderState,
        model: Matrix4<f32>,
        view: Matrix4<f32>,
        proj: Matrix4<f32>,
        image_index: usize,
    ) -> Result<()> {
        for i in 0..render_state.descriptor_set.rrdata.len() {
            let rrdata = &mut render_state.descriptor_set.rrdata[i];

            let ubo = UniformBufferObject { model, view, proj };
            rrdata.rruniform_buffers[image_index].update(self.device, &ubo)?;
        }

        Ok(())
    }

    fn acceleration_structure(&mut self) -> &mut Option<RRAccelerationStructure> {
        &mut self.raytracing.acceleration_structure
    }

    pub unsafe fn upload_mesh_vertices(&mut self, mesh_id: MeshId) -> Result<()> {
        self.graphics.upload_mesh_vertices(
            self.instance,
            self.device,
            self.command_pool.as_ref(),
            mesh_id,
        )
    }

    pub unsafe fn update_acceleration_structure(&mut self, mesh_ids: &[MeshId]) -> Result<()> {
        let Some(ref mut accel_struct) = self.raytracing.acceleration_structure else {
            return Ok(());
        };

        let blas_index_of_mesh = collect_blas_index_of_mesh(self.graphics);

        for &mesh_id in mesh_ids {
            let Some(blas_index) = blas_index_of_mesh.get(mesh_id).copied().flatten() else {
                continue;
            };
            if blas_index >= accel_struct.blas_list.len() {
                continue;
            }

            let mesh = &self.graphics.meshes[mesh_id];
            let blas = &mut accel_struct.blas_list[blas_index];

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

    pub unsafe fn rebuild_tlas(&mut self) -> Result<()> {
        let Some(ref mut accel_struct) = self.raytracing.acceleration_structure else {
            return Ok(());
        };

        let tlas = &mut accel_struct.tlas;
        RRAccelerationStructure::update_tlas(
            self.instance,
            self.device,
            self.command_pool.as_ref(),
            tlas,
            &accel_struct.blas_list,
            &accel_struct.procedural_blas,
        )?;

        Ok(())
    }

    pub unsafe fn create_static_line_buffers(
        &mut self,
        mesh: &mut StaticLineMesh,
        memory_type: BufferMemoryType,
    ) -> Result<()> {
        mesh.vertex_buffer_handle = match memory_type {
            BufferMemoryType::DeviceLocal => self.buffer_registry.create_vertex_buffer(
                self.instance,
                self.device,
                self.command_pool.as_ref(),
                &mesh.vertices,
                BufferMemoryType::DeviceLocal,
            )?,
            BufferMemoryType::HostVisible => self
                .buffer_registry
                .create_host_visible_vertex_buffer(self.instance, self.device, &mesh.vertices, 0)?,
        };

        mesh.index_buffer_handle = self.buffer_registry.create_index_buffer(
            self.instance,
            self.device,
            self.command_pool.as_ref(),
            &mesh.indices,
        )?;

        Ok(())
    }

    pub unsafe fn destroy_static_line_buffers(&mut self, mesh: &mut StaticLineMesh) {
        if mesh.vertex_buffer_handle.is_valid() {
            self.buffer_registry
                .destroy_vertex_buffer(self.device, mesh.vertex_buffer_handle);
            mesh.vertex_buffer_handle = VertexBufferHandle::INVALID;
        }
        if mesh.index_buffer_handle.is_valid() {
            self.buffer_registry
                .destroy_index_buffer(self.device, mesh.index_buffer_handle);
            mesh.index_buffer_handle = IndexBufferHandle::INVALID;
        }
    }

    pub unsafe fn update_or_create_line_buffers(
        &mut self,
        mesh: &mut LineMesh,
        frame_slot: usize,
    ) -> Result<()> {
        if mesh.vertices.is_empty() {
            return Ok(());
        }

        let vertex_data_size = (std::mem::size_of_val(mesh.vertices.as_slice())) as u64;
        let vhandle = &mut mesh.vertex_buffer_handles[frame_slot];

        if !vhandle.is_valid()
            || self.buffer_registry.get_vertex_buffer_size(*vhandle) < vertex_data_size
        {
            if vhandle.is_valid() {
                self.buffer_registry
                    .destroy_vertex_buffer(self.device, *vhandle);
            }
            let vertex_handle = self.buffer_registry.create_host_visible_vertex_buffer(
                self.instance,
                self.device,
                &mesh.vertices,
                1024,
            )?;
            *vhandle = vertex_handle;
        } else {
            self.buffer_registry
                .update_vertex_buffer(self.device, *vhandle, &mesh.vertices)?;
        }

        let index_data_size = (std::mem::size_of::<u32>() * mesh.indices.len()) as u64;
        let ihandle = &mut mesh.index_buffer_handles[frame_slot];

        if !ihandle.is_valid()
            || self.buffer_registry.get_index_buffer_size(*ihandle) < index_data_size
        {
            if ihandle.is_valid() {
                self.buffer_registry
                    .destroy_index_buffer(self.device, *ihandle);
            }
            let index_handle = self.buffer_registry.create_host_visible_index_buffer(
                self.instance,
                self.device,
                &mesh.indices,
            )?;
            *ihandle = index_handle;
        } else {
            self.buffer_registry
                .update_index_buffer(self.device, *ihandle, &mesh.indices)?;
        }

        Ok(())
    }

    pub unsafe fn destroy_line_buffers(&mut self, mesh: &mut LineMesh) {
        for slot in 0..FRAMES_IN_FLIGHT {
            if mesh.vertex_buffer_handles[slot].is_valid() {
                self.buffer_registry
                    .destroy_vertex_buffer(self.device, mesh.vertex_buffer_handles[slot]);
                mesh.vertex_buffer_handles[slot] = VertexBufferHandle::INVALID;
            }
            if mesh.index_buffer_handles[slot].is_valid() {
                self.buffer_registry
                    .destroy_index_buffer(self.device, mesh.index_buffer_handles[slot]);
                mesh.index_buffer_handles[slot] = IndexBufferHandle::INVALID;
            }
        }
    }

    pub unsafe fn update_frame_ubo(
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

    pub unsafe fn update_object_ubo(
        &mut self,
        model_matrix: Matrix4<f32>,
        object_index: usize,
        image_index: usize,
    ) -> Result<()> {
        let ubo = ObjectUBO {
            model: model_matrix,
        };
        self.graphics
            .objects
            .update(self.device, image_index, object_index, &ubo)?;
        Ok(())
    }

    pub unsafe fn update_scene_uniform(
        &mut self,
        view: Matrix4<f32>,
        proj: Matrix4<f32>,
        light_pos: Vector3<f32>,
        light_color: Vector3<f32>,
        debug_mode: i32,
        shadow_strength: f32,
        distance_attenuation: DistanceAttenuation,
        exposure_value: f32,
    ) -> Result<()> {
        let Some(scene_uniform_buffer) = self.raytracing.scene_uniform_buffer.as_ref() else {
            return Ok(());
        };

        let scene_data = SceneUniformData {
            light_position: thyllore_math_core::Vec4::new(
                light_pos.x,
                light_pos.y,
                light_pos.z,
                1.0,
            ),
            light_color: thyllore_math_core::Vec4::new(
                light_color.x,
                light_color.y,
                light_color.z,
                thyllore_render_core::AMBIENT_INTENSITY,
            ),
            view,
            proj,
            debug_mode,
            shadow_strength,
            enable_distance_attenuation: distance_attenuation.as_int(),
            exposure_value,
        };

        scene_uniform_buffer.write_slot(&self.device, 0, &scene_data)
    }
}

/// BLAS list holds only gbuffer meshes, so mesh index and BLAS index diverge once a mesh is hidden.
fn collect_blas_index_of_mesh(graphics: &GraphicsResources) -> Vec<Option<usize>> {
    let mut next_blas_index = 0;

    graphics
        .meshes
        .iter()
        .map(|mesh| {
            if !mesh.render_to_gbuffer {
                return None;
            }

            let blas_index = next_blas_index;
            next_blas_index += 1;
            Some(blas_index)
        })
        .collect()
}
