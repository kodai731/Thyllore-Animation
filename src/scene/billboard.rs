use std::mem::size_of;

use anyhow::Result;
use cgmath::{InnerSpace, Matrix4, SquareMatrix, Vector3, Vector4};
use vulkanalia::prelude::v1_0::*;

use crate::ecs::RenderData;
use crate::vulkanr::buffer::create_buffer;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::descriptor::RRBillboardDescriptorSet;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::image::RRImage;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::vulkan::Instance;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct BillboardVertex {
    pub pos: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl BillboardVertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<BillboardVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();
        let tex_coord = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(std::mem::size_of::<[f32; 3]>() as u32)
            .build();
        [pos, tex_coord]
    }
}

#[derive(Clone, Debug, Default)]
pub struct BillboardData {
    pub pipeline: RRPipeline,
    pub descriptor_set: RRBillboardDescriptorSet,
    pub transform: Option<BillboardTransform>,
    pub object_index: usize,
    pub vertices: Vec<BillboardVertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Option<vk::Buffer>,
    pub vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub index_buffer: Option<vk::Buffer>,
    pub index_buffer_memory: Option<vk::DeviceMemory>,
    pub texture: Option<RRImage>,
}

#[derive(Clone, Debug)]
pub struct BillboardTransform {
    pub position: Vector3<f32>,
    pub model_matrix: Matrix4<f32>,
}

impl BillboardData {
    pub fn new() -> Self {
        let billboard_size = 0.5;
        let vertices = vec![
            BillboardVertex {
                pos: [-billboard_size, -billboard_size, 0.0],
                tex_coord: [0.0, 1.0],
            },
            BillboardVertex {
                pos: [billboard_size, -billboard_size, 0.0],
                tex_coord: [1.0, 1.0],
            },
            BillboardVertex {
                pos: [billboard_size, billboard_size, 0.0],
                tex_coord: [1.0, 0.0],
            },
            BillboardVertex {
                pos: [-billboard_size, billboard_size, 0.0],
                tex_coord: [0.0, 0.0],
            },
        ];

        let indices = vec![0, 1, 2, 0, 2, 3];

        Self {
            pipeline: RRPipeline::default(),
            descriptor_set: RRBillboardDescriptorSet::default(),
            transform: None,
            object_index: 0,
            vertices,
            indices,
            vertex_buffer: None,
            vertex_buffer_memory: None,
            index_buffer: None,
            index_buffer_memory: None,
            texture: None,
        }
    }

    pub unsafe fn create_buffers(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
    ) -> Result<()> {
        let vertex_buffer_size = (size_of::<BillboardVertex>() * self.vertices.len()) as u64;
        let (vertex_buffer, vertex_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = rrdevice.device.map_memory(
            vertex_buffer_memory,
            0,
            vertex_buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(self.vertices.as_ptr(), data.cast(), self.vertices.len());
        rrdevice.device.unmap_memory(vertex_buffer_memory);

        self.vertex_buffer = Some(vertex_buffer);
        self.vertex_buffer_memory = Some(vertex_buffer_memory);

        let index_buffer_size = (size_of::<u32>() * self.indices.len()) as u64;
        let (index_buffer, index_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            index_buffer_size,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = rrdevice.device.map_memory(
            index_buffer_memory,
            0,
            index_buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(self.indices.as_ptr(), data.cast(), self.indices.len());
        rrdevice.device.unmap_memory(index_buffer_memory);

        self.index_buffer = Some(index_buffer);
        self.index_buffer_memory = Some(index_buffer_memory);

        let texture_path = std::path::Path::new("assets/textures/lightIcon.png");
        self.texture = Some(RRImage::new_from_file(
            instance,
            rrdevice,
            rrcommand_pool,
            texture_path,
        ).map_err(|e| anyhow::anyhow!("Failed to load billboard texture: {}", e))?);

        Ok(())
    }

    pub fn render_data(&self) -> RenderData {
        let model_matrix = self
            .transform
            .as_ref()
            .map(|t| t.model_matrix)
            .unwrap_or_else(Matrix4::identity);

        RenderData {
            object_index: self.object_index,
            pipeline: self.pipeline.clone(),
            vertex_buffer: self.vertex_buffer.unwrap_or(vk::Buffer::null()),
            index_buffer: self.index_buffer.unwrap_or(vk::Buffer::null()),
            index_count: self.indices.len() as u32,
            model_matrix,
        }
    }
}

impl BillboardTransform {
    pub fn new(position: Vector3<f32>) -> Self {
        Self {
            position,
            model_matrix: Matrix4::from_translation(position),
        }
    }

    pub fn update_look_at(&mut self, camera_position: Vector3<f32>, world_up: Vector3<f32>) {
        let forward = (camera_position - self.position).normalize();
        let right = world_up.cross(forward).normalize();
        let up = forward.cross(right);

        let rotation = Matrix4::from_cols(
            right.extend(0.0),
            up.extend(0.0),
            forward.extend(0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );

        let translation = Matrix4::from_translation(self.position);
        self.model_matrix = translation * rotation;
    }

    pub fn set_position(&mut self, position: Vector3<f32>) {
        self.position = position;
    }
}
