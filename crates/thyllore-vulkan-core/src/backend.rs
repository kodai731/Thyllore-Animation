use std::mem::size_of;
use std::rc::Rc;

use anyhow::Result;
use glam::{Mat4, Vec3, Vec4};

use thyllore_render_core::{
    BillboardMesh, BufferMemoryType, DistanceAttenuation, FrameUBO, IndexBufferHandle, LineMesh,
    MeshId, ObjectUBO, ProjectionData, StaticLineMesh, VertexBufferHandle, FRAMES_IN_FLIGHT,
};

use vulkanalia::prelude::v1_0::*;

use crate::command::RRCommandPool;
use crate::core::RRDevice;
use crate::data::{SceneUniformData, UniformBufferObject, VertexGeometry};
use crate::pipeline::RRPipeline;
use crate::raytracing::RRAccelerationStructure;
use crate::renderer::record_skinning_dispatch;
use crate::resource::GraphicsResources;
use crate::resource::RRImage;
use crate::resource::RayTracingData;
use crate::resource::{BillboardRenderState, GpuBufferRegistry, SkinStreamsUpload};
use crate::types::{CommandBuffer, Instance};

/// How the device geometry of an animated mesh is brought up to date in the frame's command buffer.
#[derive(Clone, Copy, Debug)]
pub enum GeometryUpload {
    StagedVertices,
    Skinning(SkinStreamsUpload),
}

#[derive(Clone, Copy, Debug)]
pub struct AnimatedMeshUpload {
    pub mesh_id: MeshId,
    pub geometry: GeometryUpload,
}

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
        model: Mat4,
        view: Mat4,
        proj: Mat4,
        image_index: usize,
    ) -> Result<()> {
        for i in 0..render_state.descriptor_set.rrdata.len() {
            let rrdata = &mut render_state.descriptor_set.rrdata[i];

            let ubo = UniformBufferObject { model, view, proj };
            rrdata.rruniform_buffers[image_index].update(self.device, &ubo)?;
        }

        Ok(())
    }

    pub unsafe fn stage_animated_meshes(
        &mut self,
        mesh_ids: &[MeshId],
        frame_slot: usize,
    ) -> Result<Vec<AnimatedMeshUpload>> {
        let mut uploads = Vec::with_capacity(mesh_ids.len());
        for &mesh_id in mesh_ids {
            let Some(mesh) = self.graphics.meshes.get_mut(mesh_id) else {
                continue;
            };

            let geometry = if mesh.is_skinned_on_device() {
                GeometryUpload::Skinning(mesh.stage_skinning(
                    self.instance,
                    self.device,
                    frame_slot,
                )?)
            } else {
                mesh.stage_geometry(self.instance, self.device, frame_slot)?;
                GeometryUpload::StagedVertices
            };
            uploads.push(AnimatedMeshUpload { mesh_id, geometry });
        }

        Ok(uploads)
    }

    pub unsafe fn upload_mesh_attributes(&mut self, mesh_ids: &[MeshId]) -> Result<()> {
        for &mesh_id in mesh_ids {
            let Some(mesh) = self.graphics.meshes.get_mut(mesh_id) else {
                continue;
            };
            mesh.upload_attributes(self.instance, self.device, self.command_pool.as_ref())?;
        }

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
        camera_pos: Vec3,
        light_pos: Vec3,
        light_color: Vec3,
        image_index: usize,
    ) -> Result<()> {
        let ubo = FrameUBO {
            view: proj_data.view,
            proj: proj_data.proj,
            camera_pos: Vec4::new(camera_pos.x, camera_pos.y, camera_pos.z, 1.0),
            light_pos: Vec4::new(light_pos.x, light_pos.y, light_pos.z, 1.0),
            light_color: Vec4::new(light_color.x, light_color.y, light_color.z, 1.0),
        };

        self.graphics
            .frame_set
            .update(self.device, image_index, &ubo)?;

        Ok(())
    }

    pub unsafe fn update_object_ubo(
        &mut self,
        model_matrix: Mat4,
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
        view: Mat4,
        proj: Mat4,
        light_pos: Vec3,
        light_color: Vec3,
        debug_mode: i32,
        shadow_strength: f32,
        distance_attenuation: DistanceAttenuation,
        exposure_value: f32,
    ) -> Result<()> {
        let Some(scene_uniform_buffer) = self.raytracing.scene_uniform_buffer.as_ref() else {
            return Ok(());
        };

        let scene_data = SceneUniformData {
            light_position: glam::Vec4::new(light_pos.x, light_pos.y, light_pos.z, 1.0),
            light_color: glam::Vec4::new(
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
    device.device().cmd_pipeline_barrier(
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
            &mesh.geometry_buffer.buffer,
            mesh.vertex_data.vertices.len() as u32,
            size_of::<VertexGeometry>() as u32,
            &mesh.index_buffer.buffer,
            mesh.vertex_data.indices.len() as u32,
            command_buffer,
        )?;
    }

    Ok(())
}

type StageAccess = (vk::PipelineStageFlags, vk::AccessFlags);

fn geometry_stream_writes() -> StageAccess {
    (
        vk::PipelineStageFlags::TRANSFER | vk::PipelineStageFlags::COMPUTE_SHADER,
        vk::AccessFlags::TRANSFER_WRITE | vk::AccessFlags::SHADER_WRITE,
    )
}

fn geometry_and_acceleration_writes() -> StageAccess {
    let (stages, accesses) = geometry_stream_writes();
    (
        stages | vk::PipelineStageFlags::ACCELERATION_STRUCTURE_BUILD_KHR,
        accesses | vk::AccessFlags::ACCELERATION_STRUCTURE_WRITE_KHR,
    )
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
        geometry_stream_writes(),
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

unsafe fn record_stream_copies(
    device: &RRDevice,
    graphics: &GraphicsResources,
    uploads: &[AnimatedMeshUpload],
    frame_slot: usize,
    command_buffer: vk::CommandBuffer,
) {
    for upload in uploads {
        let Some(mesh) = graphics.meshes.get(upload.mesh_id) else {
            continue;
        };
        match upload.geometry {
            GeometryUpload::StagedVertices => {
                mesh.record_geometry_copy(frame_slot, device.device(), command_buffer)
            }
            GeometryUpload::Skinning(SkinStreamsUpload::Staged) => {
                mesh.record_skin_stream_copies(frame_slot, device.device(), command_buffer)
            }
            GeometryUpload::Skinning(SkinStreamsUpload::Unchanged) => {}
        }
    }
}

unsafe fn record_skinning_dispatches(
    device: &RRDevice,
    graphics: &GraphicsResources,
    skinning_pipeline: &RRPipeline,
    uploads: &[AnimatedMeshUpload],
    frame_slot: usize,
    command_buffer: vk::CommandBuffer,
) {
    record_global_memory_barrier(
        device,
        command_buffer,
        (
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::TRANSFER_WRITE,
        ),
        (
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::AccessFlags::SHADER_READ,
        ),
    );

    for upload in uploads {
        let GeometryUpload::Skinning(_) = upload.geometry else {
            continue;
        };
        let Some(mesh) = graphics.meshes.get(upload.mesh_id) else {
            continue;
        };
        let Some(streams) = mesh.skin_stream_addresses(device, frame_slot) else {
            continue;
        };

        record_skinning_dispatch(
            device,
            command_buffer,
            skinning_pipeline,
            streams,
            device.buffer_address(mesh.geometry_buffer.buffer),
            mesh.vertex_data.vertices.len() as u32,
        );
    }
}

pub unsafe fn record_animated_mesh_uploads(
    instance: &Instance,
    device: &RRDevice,
    graphics: &GraphicsResources,
    raytracing: &mut RayTracingData,
    skinning_pipeline: &RRPipeline,
    uploads: &[AnimatedMeshUpload],
    frame_slot: usize,
    command_buffer: CommandBuffer,
) -> Result<()> {
    let command_buffer = command_buffer.raw();

    record_global_memory_barrier(
        device,
        command_buffer,
        (
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::empty(),
        ),
        geometry_and_acceleration_writes(),
    );

    record_stream_copies(device, graphics, uploads, frame_slot, command_buffer);
    record_skinning_dispatches(
        device,
        graphics,
        skinning_pipeline,
        uploads,
        frame_slot,
        command_buffer,
    );

    if let Some(accel_struct) = raytracing.acceleration_structure.as_mut() {
        let mesh_ids: Vec<MeshId> = uploads.iter().map(|upload| upload.mesh_id).collect();
        record_acceleration_structure_updates(
            instance,
            device,
            graphics,
            accel_struct,
            &mesh_ids,
            frame_slot,
            command_buffer,
        )?;
    }

    record_global_memory_barrier(
        device,
        command_buffer,
        geometry_and_acceleration_writes(),
        (
            vk::PipelineStageFlags::VERTEX_INPUT
                | vk::PipelineStageFlags::VERTEX_SHADER
                | vk::PipelineStageFlags::FRAGMENT_SHADER
                | vk::PipelineStageFlags::COMPUTE_SHADER
                | vk::PipelineStageFlags::RAY_TRACING_SHADER_KHR,
            vk::AccessFlags::VERTEX_ATTRIBUTE_READ
                | vk::AccessFlags::SHADER_READ
                | vk::AccessFlags::ACCELERATION_STRUCTURE_READ_KHR,
        ),
    );

    Ok(())
}
