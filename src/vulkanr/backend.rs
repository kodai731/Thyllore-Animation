use std::ffi::c_void;
use std::mem::size_of;
use std::rc::Rc;

use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::render::{MeshId, RenderBackend};
use crate::scene::graphics_resource::GraphicsResources;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::core::device::RRDevice;
use crate::vulkanr::data::Vertex;
use crate::vulkanr::raytracing::acceleration::RRAccelerationStructure;
use crate::vulkanr::vulkan::Instance;

pub struct VulkanBackend<'a> {
    pub instance: &'a Instance,
    pub device: &'a RRDevice,
    pub command_pool: Rc<RRCommandPool>,
    pub graphics: &'a mut GraphicsResources,
    pub acceleration_structure: &'a mut Option<RRAccelerationStructure>,
}

impl<'a> VulkanBackend<'a> {
    pub fn new(
        instance: &'a Instance,
        device: &'a RRDevice,
        command_pool: Rc<RRCommandPool>,
        graphics: &'a mut GraphicsResources,
        acceleration_structure: &'a mut Option<RRAccelerationStructure>,
    ) -> Self {
        Self {
            instance,
            device,
            command_pool,
            graphics,
            acceleration_structure,
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
}
