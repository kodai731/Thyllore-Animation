use super::buffer::*;
use super::device::*;
use super::swapchain::*;
use super::vulkan::*;
use crate::math::*;
use std::cmp::PartialEq;
use std::hash::{Hash, Hasher};

#[repr(C)] // for compatibility of C struct
#[derive(Copy, Clone, Debug, Default)]
pub struct Vertex {
    pub pos: Vec3,
    pub color: Vec4,
    pub tex_coord: Vec2,
    pub normal: Vec3,
}

#[derive(Clone, Debug, Default)]
pub struct VertexData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

// TODO: implement iterator
#[derive(Clone, Debug, Default)]
pub struct RRData {
    pub rruniform_buffers: Vec<RRUniformBuffer>,
    pub image: vk::Image,
    pub image_memory: vk::DeviceMemory,
    pub mip_level: u32,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub vertex_data: VertexData,
    pub vertex_buffer: RRVertexBuffer,
    pub index_buffer: RRIndexBuffer,
}

impl RRData {
    pub unsafe fn new(instance: &Instance, rrdevice: &RRDevice, rrswapchain: &RRSwapchain) -> Self {
        let mut rrdata = RRData::default();
        Self::create_uniform_buffers(&mut rrdata, instance, rrdevice, rrswapchain);
        rrdata
    }

    pub unsafe fn create_uniform_buffers(
        rrdata: &mut RRData,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
    ) {
        for _ in 0..rrswapchain.swapchain_images.len() {
            let ubo = UniformBufferObject::default();
            let rruniform_buffer = RRUniformBuffer::new(instance, rrdevice, ubo);
            rrdata.rruniform_buffers.push(rruniform_buffer);
        }
    }

    pub unsafe fn update_ubo(
        &mut self,
        rrdevice: &RRDevice,
        image_index: usize,
        ubo: &UniformBufferObject,
    ) -> Result<(), anyhow::Error> {
        use std::ptr::copy_nonoverlapping as memcpy;

        let ubo_memory = self.rruniform_buffers[image_index].buffer_memory;
        let memory = rrdevice.device.map_memory(
            ubo_memory,
            0,
            std::mem::size_of::<UniformBufferObject>() as u64,
            vk::MemoryMapFlags::empty(),
        )?;
        memcpy(ubo, memory.cast(), 1);
        rrdevice.device.unmap_memory(ubo_memory);
        Ok(())
    }

    pub unsafe fn delete_buffers(&mut self, rrdevice: &RRDevice) {
        for uniform_buffer in &self.rruniform_buffers {
            uniform_buffer.delete(rrdevice);
        }
        self.rruniform_buffers.clear();

        if self.image_view != vk::ImageView::null() {
            rrdevice.device.destroy_image_view(self.image_view, None);
            self.image_view = vk::ImageView::null();
        }

        if self.sampler != vk::Sampler::null() {
            rrdevice.device.destroy_sampler(self.sampler, None);
            self.sampler = vk::Sampler::null();
        }

        if self.vertex_buffer.buffer != vk::Buffer::null() {
            rrdevice.device.destroy_buffer(self.vertex_buffer.buffer, None);
            self.vertex_buffer.buffer = vk::Buffer::null();
        }

        if self.vertex_buffer.buffer_memory != vk::DeviceMemory::null() {
            rrdevice.device.free_memory(self.vertex_buffer.buffer_memory, None);
            self.vertex_buffer.buffer_memory = vk::DeviceMemory::null();
        }

        if self.index_buffer.buffer != vk::Buffer::null() {
            rrdevice.device.destroy_buffer(self.index_buffer.buffer, None);
            self.index_buffer.buffer = vk::Buffer::null();
        }

        if self.index_buffer.buffer_memory != vk::DeviceMemory::null() {
            rrdevice.device.free_memory(self.index_buffer.buffer_memory, None);
            self.index_buffer.buffer_memory = vk::DeviceMemory::null();
        }
    }

    pub unsafe fn delete(&mut self, rrdevice: &RRDevice) {
        self.delete_buffers(rrdevice);

        if self.image != vk::Image::null() {
            rrdevice.device.destroy_image(self.image, None);
            self.image = vk::Image::null();
        }

        if self.image_memory != vk::DeviceMemory::null() {
            rrdevice.device.free_memory(self.image_memory, None);
            self.image_memory = vk::DeviceMemory::null();
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UniformBufferObject {
    pub model: Mat4,
    pub view: Mat4,
    pub proj: Mat4,
}

impl Default for UniformBufferObject {
    fn default() -> Self {
        let identity = Mat4::identity();
        Self {
            model: identity,
            view: identity,
            proj: identity,
        }
    }
}

/// Scene data for ray query and composite shaders
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SceneUniformData {
    pub light_position: Vec4,
    pub light_color: Vec4,
    pub view: Mat4,
    pub proj: Mat4,
    pub debug_mode: i32,
    pub shadow_strength: f32,
    pub enable_distance_attenuation: i32,
    pub _padding: i32,
}

impl Default for SceneUniformData {
    fn default() -> Self {
        let identity = Mat4::identity();
        Self {
            light_position: Vec4::new(5.0, 5.0, 5.0, 1.0),
            light_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            view: identity,
            proj: identity,
            debug_mode: 0,
            shadow_strength: 1.0,
            enable_distance_attenuation: 0,
            _padding: 0,
        }
    }
}

impl PartialEq for Vertex {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos && self.color == other.color && self.tex_coord == other.tex_coord && self.normal == other.normal
    }
}

impl Eq for Vertex {}

impl Hash for Vertex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pos[0].to_bits().hash(state);
        self.pos[1].to_bits().hash(state);
        self.pos[2].to_bits().hash(state);
        self.color[0].to_bits().hash(state);
        self.color[1].to_bits().hash(state);
        self.color[2].to_bits().hash(state);
        self.tex_coord[0].to_bits().hash(state);
        self.tex_coord[1].to_bits().hash(state);
        self.normal[0].to_bits().hash(state);
        self.normal[1].to_bits().hash(state);
        self.normal[2].to_bits().hash(state);
    }
}
impl Vertex {
    pub fn new(pos: Vec3, color: Vec4, tex_coord: Vec2) -> Self {
        Self {
            pos,
            color,
            tex_coord,
            normal: Vec3::new(0.0, 0.0, 1.0), // Default normal pointing up
        }
    }

    pub fn new_with_normal(pos: Vec3, color: Vec4, tex_coord: Vec2, normal: Vec3) -> Self {
        Self {
            pos,
            color,
            tex_coord,
            normal,
        }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        //  at which rate to load data from memory throughout the vertices
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 4] {
        // how to extract a vertex attribute from a chunk of vertex data originating from a binding description
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0) // directive of the input in the vertex shader
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(size_of::<Vec3>() as u32)
            .build();

        let tex_coord = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset((size_of::<Vec3>() + size_of::<Vec4>()) as u32)
            .build();

        let normal = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(3)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset((size_of::<Vec3>() + size_of::<Vec4>() + size_of::<Vec2>()) as u32)
            .build();

        [pos, color, tex_coord, normal]
    }
}
