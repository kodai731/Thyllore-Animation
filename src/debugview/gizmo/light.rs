use super::grid::GizmoVertex;
use crate::vulkanr::buffer::*;
use crate::vulkanr::command::*;
use crate::vulkanr::device::*;
use crate::vulkanr::vulkan::*;
use crate::vulkanr::image::*;
use cgmath::Vector3;
use std::mem::size_of;

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
            .stride(size_of::<BillboardVertex>() as u32)
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
            .offset(size_of::<[f32; 3]>() as u32)
            .build();
        [pos, tex_coord]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum LightGizmoAxis {
    #[default]
    None,
    X,
    Y,
    Z,
    Center,
}

#[derive(Clone, Debug)]
pub struct LightGizmoData {
    pub position: Vector3<f32>,
    pub vertices: Vec<GizmoVertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Option<vk::Buffer>,
    pub vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub index_buffer: Option<vk::Buffer>,
    pub index_buffer_memory: Option<vk::DeviceMemory>,
    pub selected_axis: LightGizmoAxis,
    pub billboard_vertices: Vec<BillboardVertex>,
    pub billboard_indices: Vec<u32>,
    pub billboard_vertex_buffer: Option<vk::Buffer>,
    pub billboard_vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub billboard_index_buffer: Option<vk::Buffer>,
    pub billboard_index_buffer_memory: Option<vk::DeviceMemory>,
    pub billboard_texture: Option<RRImage>,
}

impl Default for LightGizmoData {
    fn default() -> Self {
        Self::new(Vector3::new(0.0, 0.0, 0.0))
    }
}

impl LightGizmoData {
    pub fn new(position: Vector3<f32>) -> Self {
        let axis_length = 1.0;
        let yellow = [1.0, 1.0, 0.0];

        let vertices = vec![
            GizmoVertex { pos: [0.0, 0.0, 0.0], color: yellow },
            GizmoVertex { pos: [axis_length, 0.0, 0.0], color: [1.0, 0.0, 0.0] },
            GizmoVertex { pos: [0.0, axis_length, 0.0], color: [0.0, 1.0, 0.0] },
            GizmoVertex { pos: [0.0, 0.0, axis_length], color: [0.0, 0.0, 1.0] },
        ];

        let indices = vec![
            0, 1,
            0, 2,
            0, 3,
        ];

        let billboard_size = 0.5;
        let billboard_vertices = vec![
            BillboardVertex { pos: [-billboard_size, -billboard_size, 0.0], tex_coord: [0.0, 1.0] },
            BillboardVertex { pos: [billboard_size, -billboard_size, 0.0], tex_coord: [1.0, 1.0] },
            BillboardVertex { pos: [billboard_size, billboard_size, 0.0], tex_coord: [1.0, 0.0] },
            BillboardVertex { pos: [-billboard_size, billboard_size, 0.0], tex_coord: [0.0, 0.0] },
        ];

        let billboard_indices = vec![
            0, 1, 2,
            0, 2, 3,
        ];

        Self {
            position,
            vertices,
            indices,
            vertex_buffer: None,
            vertex_buffer_memory: None,
            index_buffer: None,
            index_buffer_memory: None,
            selected_axis: LightGizmoAxis::None,
            billboard_vertices,
            billboard_indices,
            billboard_vertex_buffer: None,
            billboard_vertex_buffer_memory: None,
            billboard_index_buffer: None,
            billboard_index_buffer_memory: None,
            billboard_texture: None,
        }
    }

    pub fn update_position(&mut self, position: Vector3<f32>) {
        self.position = position;
    }

    pub fn update_selection_color(&mut self) {
        let yellow = [1.0, 1.0, 0.0];
        let highlight = [1.0, 1.0, 0.5];

        self.vertices[0].color = yellow;
        self.vertices[1].color = [1.0, 0.0, 0.0];
        self.vertices[2].color = [0.0, 1.0, 0.0];
        self.vertices[3].color = [0.0, 0.0, 1.0];

        match self.selected_axis {
            LightGizmoAxis::None => {}
            LightGizmoAxis::Center => {
                self.vertices[0].color = highlight;
            }
            LightGizmoAxis::X => {
                self.vertices[1].color = [1.0, 0.5, 0.0];
            }
            LightGizmoAxis::Y => {
                self.vertices[2].color = [0.5, 1.0, 0.0];
            }
            LightGizmoAxis::Z => {
                self.vertices[3].color = [0.0, 0.5, 1.0];
            }
        }
    }

    pub unsafe fn create_buffers(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let vertex_buffer_size = (size_of::<GizmoVertex>() * self.vertices.len()) as u64;
        let (vertex_buffer, vertex_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = rrdevice
            .device
            .map_memory(vertex_buffer_memory, 0, vertex_buffer_size, vk::MemoryMapFlags::empty())?;
        std::ptr::copy_nonoverlapping(self.vertices.as_ptr(), data.cast(), self.vertices.len());
        rrdevice.device.unmap_memory(vertex_buffer_memory);

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

    pub unsafe fn create_billboard_buffers(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let vertex_buffer_size = (size_of::<BillboardVertex>() * self.billboard_vertices.len()) as u64;
        let (vertex_buffer, vertex_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = rrdevice
            .device
            .map_memory(vertex_buffer_memory, 0, vertex_buffer_size, vk::MemoryMapFlags::empty())?;
        std::ptr::copy_nonoverlapping(self.billboard_vertices.as_ptr(), data.cast(), self.billboard_vertices.len());
        rrdevice.device.unmap_memory(vertex_buffer_memory);

        self.billboard_vertex_buffer = Some(vertex_buffer);
        self.billboard_vertex_buffer_memory = Some(vertex_buffer_memory);

        let index_buffer_size = (size_of::<u32>() * self.billboard_indices.len()) as u64;
        let (index_buffer, index_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            index_buffer_size,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = rrdevice
            .device
            .map_memory(index_buffer_memory, 0, index_buffer_size, vk::MemoryMapFlags::empty())?;
        std::ptr::copy_nonoverlapping(self.billboard_indices.as_ptr(), data.cast(), self.billboard_indices.len());
        rrdevice.device.unmap_memory(index_buffer_memory);

        self.billboard_index_buffer = Some(index_buffer);
        self.billboard_index_buffer_memory = Some(index_buffer_memory);

        let texture_path = std::path::Path::new("assets/textures/lightIcon.png");
        self.billboard_texture = Some(RRImage::new_from_file(
            instance,
            rrdevice,
            rrcommand_pool,
            texture_path,
        )?);

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

        if let Some(billboard_vertex_buffer) = self.billboard_vertex_buffer {
            rrdevice.device.destroy_buffer(billboard_vertex_buffer, None);
        }
        if let Some(billboard_vertex_buffer_memory) = self.billboard_vertex_buffer_memory {
            rrdevice.device.free_memory(billboard_vertex_buffer_memory, None);
        }
        if let Some(billboard_index_buffer) = self.billboard_index_buffer {
            rrdevice.device.destroy_buffer(billboard_index_buffer, None);
        }
        if let Some(billboard_index_buffer_memory) = self.billboard_index_buffer_memory {
            rrdevice.device.free_memory(billboard_index_buffer_memory, None);
        }

        if let Some(ref mut billboard_texture) = self.billboard_texture {
            billboard_texture.destroy(rrdevice);
        }

        self.vertex_buffer = None;
        self.vertex_buffer_memory = None;
        self.index_buffer = None;
        self.index_buffer_memory = None;
        self.billboard_vertex_buffer = None;
        self.billboard_vertex_buffer_memory = None;
        self.billboard_index_buffer = None;
        self.billboard_index_buffer_memory = None;
        self.billboard_texture = None;
    }
}
