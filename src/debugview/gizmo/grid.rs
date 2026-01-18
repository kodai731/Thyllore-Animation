use crate::ecs::RenderData;
use crate::vulkanr::buffer::*;
use crate::vulkanr::command::*;
use crate::vulkanr::device::*;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::vulkan::*;
use cgmath::{vec3, Matrix3, Matrix4, SquareMatrix};
use std::mem::size_of;
use vulkanalia::prelude::v1_0::*;

/// Gizmo用の頂点構造（位置 + 色）
#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct GizmoVertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
}

impl GizmoVertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<GizmoVertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: size_of::<[f32; 3]>() as u32,
            },
        ]
    }
}

#[derive(Clone, Debug, Default)]
pub struct GridGizmoData {
    pub pipeline: RRPipeline,
    pub vertices: Vec<GizmoVertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Option<vk::Buffer>,
    pub vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub index_buffer: Option<vk::Buffer>,
    pub index_buffer_memory: Option<vk::DeviceMemory>,
    pub object_index: usize,
}

impl GridGizmoData {
    pub fn new() -> Self {
        let axis_length = 0.15;

        let vertices = vec![
            GizmoVertex { pos: [0.0, 0.0, 0.0], color: [1.0, 1.0, 1.0] },
            GizmoVertex { pos: [axis_length, 0.0, 0.0], color: [1.0, 0.0, 0.0] },
            GizmoVertex { pos: [0.0, axis_length, 0.0], color: [0.0, 1.0, 0.0] },
            GizmoVertex { pos: [0.0, 0.0, axis_length], color: [0.0, 0.0, 1.0] },
        ];

        let indices = vec![
            0, 1,
            0, 2,
            0, 3,
        ];

        Self {
            pipeline: RRPipeline::default(),
            vertices,
            indices,
            vertex_buffer: None,
            vertex_buffer_memory: None,
            index_buffer: None,
            index_buffer_memory: None,
            object_index: 0,
        }
    }

    pub fn update_rotation(&mut self, rotation_matrix: &Matrix3<f32>) {
        let axis_length = 0.15;

        let x_axis = rotation_matrix * vec3(axis_length, 0.0, 0.0);
        let y_axis = rotation_matrix * vec3(0.0, axis_length, 0.0);
        let z_axis = rotation_matrix * vec3(0.0, 0.0, axis_length);

        self.vertices[1].pos = [x_axis.x, x_axis.y, x_axis.z];
        self.vertices[2].pos = [y_axis.x, y_axis.y, y_axis.z];
        self.vertices[3].pos = [z_axis.x, z_axis.y, z_axis.z];
    }

    pub unsafe fn create_buffers(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let vertex_buffer_size = (size_of::<GizmoVertex>() * self.vertices.len()) as u64;
        let (staging_buffer, staging_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = rrdevice
            .device
            .map_memory(staging_buffer_memory, 0, vertex_buffer_size, vk::MemoryMapFlags::empty())?;
        std::ptr::copy_nonoverlapping(self.vertices.as_ptr(), data.cast(), self.vertices.len());
        rrdevice.device.unmap_memory(staging_buffer_memory);

        let (vertex_buffer, vertex_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        copy_buffer(
            rrdevice,
            rrcommand_pool,
            staging_buffer,
            vertex_buffer,
            vertex_buffer_size,
        )?;

        rrdevice.device.destroy_buffer(staging_buffer, None);
        rrdevice.device.free_memory(staging_buffer_memory, None);

        self.vertex_buffer = Some(vertex_buffer);
        self.vertex_buffer_memory = Some(vertex_buffer_memory);

        let index_buffer_size = (size_of::<u32>() * self.indices.len()) as u64;
        let (staging_buffer, staging_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            index_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = rrdevice
            .device
            .map_memory(staging_buffer_memory, 0, index_buffer_size, vk::MemoryMapFlags::empty())?;
        std::ptr::copy_nonoverlapping(self.indices.as_ptr(), data.cast(), self.indices.len());
        rrdevice.device.unmap_memory(staging_buffer_memory);

        let (index_buffer, index_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            index_buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        copy_buffer(
            rrdevice,
            rrcommand_pool,
            staging_buffer,
            index_buffer,
            index_buffer_size,
        )?;

        rrdevice.device.destroy_buffer(staging_buffer, None);
        rrdevice.device.free_memory(staging_buffer_memory, None);

        self.index_buffer = Some(index_buffer);
        self.index_buffer_memory = Some(index_buffer_memory);

        Ok(())
    }

    pub unsafe fn update_vertex_buffer(
        &self,
        rrdevice: &RRDevice,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(vertex_buffer_memory) = self.vertex_buffer_memory {
            let vertex_buffer_size = (size_of::<GizmoVertex>() * self.vertices.len()) as u64;
            let data = rrdevice
                .device
                .map_memory(vertex_buffer_memory, 0, vertex_buffer_size, vk::MemoryMapFlags::empty())?;
            std::ptr::copy_nonoverlapping(self.vertices.as_ptr(), data.cast(), self.vertices.len());
            rrdevice.device.unmap_memory(vertex_buffer_memory);
        }
        Ok(())
    }

    pub unsafe fn destroy_buffers(&mut self, rrdevice: &RRDevice) {
        if let Some(vertex_buffer) = self.vertex_buffer {
            rrdevice.device.destroy_buffer(vertex_buffer, None);
        }
        if let Some(vertex_buffer_memory) = self.vertex_buffer_memory {
            rrdevice.device.free_memory(vertex_buffer_memory, None);
        }
        if let Some(index_buffer) = self.index_buffer {
            rrdevice.device.destroy_buffer(index_buffer, None);
        }
        if let Some(index_buffer_memory) = self.index_buffer_memory {
            rrdevice.device.free_memory(index_buffer_memory, None);
        }

        self.vertex_buffer = None;
        self.vertex_buffer_memory = None;
        self.index_buffer = None;
        self.index_buffer_memory = None;
    }

    pub fn render_data(&self) -> RenderData {
        RenderData {
            object_index: self.object_index,
            pipeline: self.pipeline.clone(),
            vertex_buffer: self.vertex_buffer.unwrap_or(vk::Buffer::null()),
            index_buffer: self.index_buffer.unwrap_or(vk::Buffer::null()),
            index_count: self.indices.len() as u32,
            model_matrix: Matrix4::identity(),
        }
    }
}
