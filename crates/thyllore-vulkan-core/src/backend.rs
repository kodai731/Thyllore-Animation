use std::mem::size_of;
use std::rc::Rc;

use anyhow::Result;
use cgmath::{Matrix4, Vector3, Vector4};

use thyllore_render_core::{
    BufferMemoryType, DistanceAttenuation, FrameUBO, IndexBufferHandle, LineMesh, MeshId,
    ObjectUBO, ProjectionData, RenderBackend, StaticLineMesh, VertexBufferHandle, FRAMES_IN_FLIGHT,
};

use vulkanalia::prelude::v1_0::*;

use crate::command::{with_single_time_commands, RRCommandPool};
use crate::core::device::RRDevice;
use crate::data::{SceneUniformData, Vertex};
use crate::raytracing::RRAccelerationStructure;
use crate::resource::graphics_resource::GraphicsResources;
use crate::resource::raytracing_data::RayTracingData;
use crate::resource::GpuBufferRegistry;
use crate::vulkan::Instance;

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

    fn acceleration_structure(&mut self) -> &mut Option<RRAccelerationStructure> {
        &mut self.raytracing.acceleration_structure
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

unsafe fn stage_mesh_vertices(
    instance: &Instance,
    device: &RRDevice,
    graphics: &mut GraphicsResources,
    mesh_ids: &[MeshId],
    frame_slot: usize,
) -> Result<Vec<(MeshId, vk::DeviceSize)>> {
    let mut staged_meshes = Vec::with_capacity(mesh_ids.len());
    for &mesh_id in mesh_ids {
        let Some(mesh) = graphics.meshes.get_mut(mesh_id) else {
            continue;
        };
        let size = mesh.stage_vertices(instance, device, frame_slot)?;
        staged_meshes.push((mesh_id, size));
    }
    Ok(staged_meshes)
}

unsafe fn record_global_memory_barrier(
    device: &RRDevice,
    command_buffer: vk::CommandBuffer,
    (src_stage, src_access): (vk::PipelineStageFlags, vk::AccessFlags),
    (dst_stage, dst_access): (vk::PipelineStageFlags, vk::AccessFlags),
) {
    let barrier = vk::MemoryBarrier::builder()
        .src_access_mask(src_access)
        .dst_access_mask(dst_access)
        .build();
    device.device.cmd_pipeline_barrier(
        command_buffer,
        src_stage,
        dst_stage,
        vk::DependencyFlags::empty(),
        &[barrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[] as &[vk::ImageMemoryBarrier],
    );
}

unsafe fn record_blas_updates(
    instance: &Instance,
    device: &RRDevice,
    graphics: &GraphicsResources,
    accel_struct: &mut RRAccelerationStructure,
    mesh_ids: &[MeshId],
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    let blas_index_of_mesh = collect_blas_index_of_mesh(graphics);

    for &mesh_id in mesh_ids {
        let Some(blas_index) = blas_index_of_mesh.get(mesh_id).copied().flatten() else {
            continue;
        };
        if blas_index >= accel_struct.blas_list.len() {
            continue;
        }

        let mesh = &graphics.meshes[mesh_id];
        let blas = &mut accel_struct.blas_list[blas_index];

        RRAccelerationStructure::record_blas_update(
            instance,
            device,
            blas,
            &mesh.vertex_buffer.buffer,
            mesh.vertex_data.vertices.len() as u32,
            size_of::<Vertex>() as u32,
            &mesh.index_buffer.buffer,
            mesh.vertex_data.indices.len() as u32,
            command_buffer,
        )?;
    }

    Ok(())
}

unsafe fn record_acceleration_structure_updates(
    instance: &Instance,
    device: &RRDevice,
    graphics: &GraphicsResources,
    accel_struct: &mut RRAccelerationStructure,
    mesh_ids: &[MeshId],
    frame_slot: usize,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    record_global_memory_barrier(
        device,
        command_buffer,
        (
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::TRANSFER_WRITE,
        ),
        (
            vk::PipelineStageFlags::ACCELERATION_STRUCTURE_BUILD_KHR,
            vk::AccessFlags::SHADER_READ | vk::AccessFlags::ACCELERATION_STRUCTURE_READ_KHR,
        ),
    );

    record_blas_updates(
        instance,
        device,
        graphics,
        accel_struct,
        mesh_ids,
        command_buffer,
    )?;

    record_global_memory_barrier(
        device,
        command_buffer,
        (
            vk::PipelineStageFlags::ACCELERATION_STRUCTURE_BUILD_KHR,
            vk::AccessFlags::ACCELERATION_STRUCTURE_WRITE_KHR,
        ),
        (
            vk::PipelineStageFlags::ACCELERATION_STRUCTURE_BUILD_KHR,
            vk::AccessFlags::ACCELERATION_STRUCTURE_READ_KHR,
        ),
    );

    if accel_struct.blas_list.is_empty() && accel_struct.procedural_blas.is_empty() {
        return Ok(());
    }

    RRAccelerationStructure::record_tlas_update(
        instance,
        device,
        &mut accel_struct.tlas,
        &accel_struct.blas_list,
        &accel_struct.procedural_blas,
        frame_slot,
        command_buffer,
    )
}

impl<'a> RenderBackend for VulkanBackend<'a> {
    unsafe fn upload_animated_meshes(
        &mut self,
        mesh_ids: &[MeshId],
        frame_slot: usize,
    ) -> Result<()> {
        let staged_meshes = stage_mesh_vertices(
            self.instance,
            self.device,
            self.graphics,
            mesh_ids,
            frame_slot,
        )?;

        let instance = self.instance;
        let device = self.device;
        let graphics: &GraphicsResources = self.graphics;
        let acceleration_structure = &mut self.raytracing.acceleration_structure;

        with_single_time_commands(device, self.command_pool.command_pool, |command_buffer| {
            for &(mesh_id, size) in &staged_meshes {
                graphics.meshes[mesh_id].record_vertex_copy(
                    frame_slot,
                    &device.device,
                    command_buffer,
                    size,
                );
            }

            let Some(accel_struct) = acceleration_structure.as_mut() else {
                return Ok(());
            };
            record_acceleration_structure_updates(
                instance,
                device,
                graphics,
                accel_struct,
                mesh_ids,
                frame_slot,
                command_buffer,
            )
        })
    }

    unsafe fn create_static_line_buffers(
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

    unsafe fn destroy_static_line_buffers(&mut self, mesh: &mut StaticLineMesh) {
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

    unsafe fn update_or_create_line_buffers(
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

    unsafe fn destroy_line_buffers(&mut self, mesh: &mut LineMesh) {
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
        let ubo = ObjectUBO {
            model: model_matrix,
        };
        self.graphics
            .objects
            .update(self.device, image_index, object_index, &ubo)?;
        Ok(())
    }

    unsafe fn update_scene_uniform(
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
