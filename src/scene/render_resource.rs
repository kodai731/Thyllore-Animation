use crate::vulkanr::buffer::{RRIndexBuffer, RRVertexBuffer};
use crate::vulkanr::core::device::RRDevice;
use crate::vulkanr::data::{Vertex, VertexData};
use crate::vulkanr::resource::buffer::create_buffer;
use crate::vulkanr::resource::image::RRImage;
use crate::vulkanr::vulkan::*;
use cgmath::{Matrix4, SquareMatrix, Vector4};
use std::collections::HashMap;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct FrameUBO {
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
    pub camera_pos: Vector4<f32>,
    pub light_pos: Vector4<f32>,
    pub light_color: Vector4<f32>,
}

impl Default for FrameUBO {
    fn default() -> Self {
        Self {
            view: Matrix4::identity(),
            proj: Matrix4::identity(),
            camera_pos: Vector4::new(0.0, 0.0, 0.0, 1.0),
            light_pos: Vector4::new(0.0, 0.0, 0.0, 1.0),
            light_color: Vector4::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct ObjectUBO {
    pub model: Matrix4<f32>,
}

impl Default for ObjectUBO {
    fn default() -> Self {
        Self {
            model: Matrix4::identity(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct MaterialUBO {
    pub base_color: Vector4<f32>,
    pub metallic: f32,
    pub roughness: f32,
    pub _padding: [f32; 2],
}

impl Default for MaterialUBO {
    fn default() -> Self {
        Self {
            base_color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            metallic: 0.0,
            roughness: 0.5,
            _padding: [0.0; 2],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertex_buffer: RRVertexBuffer,
    pub index_buffer: RRIndexBuffer,
    pub vertex_data: VertexData,
    pub image: vk::Image,
    pub image_memory: vk::DeviceMemory,
    pub mip_level: u32,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub render_to_gbuffer: bool,
}

impl Default for Mesh {
    fn default() -> Self {
        Self {
            vertex_buffer: RRVertexBuffer::default(),
            index_buffer: RRIndexBuffer::default(),
            vertex_data: VertexData::default(),
            image: vk::Image::null(),
            image_memory: vk::DeviceMemory::null(),
            mip_level: 0,
            image_view: vk::ImageView::null(),
            sampler: vk::Sampler::null(),
            render_to_gbuffer: true,
        }
    }
}

impl Mesh {
    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
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

#[derive(Clone, Debug, Default)]
pub struct FrameDescriptorSet {
    pub layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
    pub sets: Vec<vk::DescriptorSet>,
    pub buffers: Vec<vk::Buffer>,
    pub buffer_memories: Vec<vk::DeviceMemory>,
}

impl FrameDescriptorSet {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        swapchain_image_count: usize,
    ) -> anyhow::Result<Self> {
        let layout = Self::create_layout(rrdevice)?;
        let pool = Self::create_pool(rrdevice, swapchain_image_count)?;
        let sets = Self::allocate_sets(rrdevice, layout, pool, swapchain_image_count)?;

        let mut buffers = Vec::with_capacity(swapchain_image_count);
        let mut buffer_memories = Vec::with_capacity(swapchain_image_count);

        for _ in 0..swapchain_image_count {
            let (buffer, memory) = create_buffer(
                instance,
                rrdevice,
                size_of::<FrameUBO>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            buffers.push(buffer);
            buffer_memories.push(memory);
        }

        let mut frame_set = Self {
            layout,
            pool,
            sets,
            buffers,
            buffer_memories,
        };
        frame_set.write_descriptor_sets(rrdevice);

        Ok(frame_set)
    }

    unsafe fn create_layout(rrdevice: &RRDevice) -> anyhow::Result<vk::DescriptorSetLayout> {
        let ubo_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT);

        let bindings = &[ubo_binding];
        let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);

        Ok(rrdevice.device.create_descriptor_set_layout(&info, None)?)
    }

    unsafe fn create_pool(
        rrdevice: &RRDevice,
        count: usize,
    ) -> anyhow::Result<vk::DescriptorPool> {
        let pool_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(count as u32);

        let pool_sizes = &[pool_size];
        let info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(pool_sizes)
            .max_sets(count as u32)
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);

        Ok(rrdevice.device.create_descriptor_pool(&info, None)?)
    }

    unsafe fn allocate_sets(
        rrdevice: &RRDevice,
        layout: vk::DescriptorSetLayout,
        pool: vk::DescriptorPool,
        count: usize,
    ) -> anyhow::Result<Vec<vk::DescriptorSet>> {
        let layouts = vec![layout; count];
        let info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool)
            .set_layouts(&layouts);

        Ok(rrdevice.device.allocate_descriptor_sets(&info)?)
    }

    unsafe fn write_descriptor_sets(&mut self, rrdevice: &RRDevice) {
        for (i, &set) in self.sets.iter().enumerate() {
            let buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.buffers[i])
                .offset(0)
                .range(size_of::<FrameUBO>() as u64);

            let buffer_infos = &[buffer_info];
            let write = vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(buffer_infos);

            rrdevice.device.update_descriptor_sets(&[write], &[] as &[vk::CopyDescriptorSet]);
        }
    }

    pub unsafe fn update(&self, rrdevice: &RRDevice, image_index: usize, ubo: &FrameUBO) -> anyhow::Result<()> {
        let memory = rrdevice.device.map_memory(
            self.buffer_memories[image_index],
            0,
            size_of::<FrameUBO>() as u64,
            vk::MemoryMapFlags::empty(),
        )?;
        memcpy(ubo, memory.cast(), 1);
        rrdevice.device.unmap_memory(self.buffer_memories[image_index]);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        for &buffer in &self.buffers {
            device.destroy_buffer(buffer, None);
        }
        for &memory in &self.buffer_memories {
            device.free_memory(memory, None);
        }

        if !self.sets.is_empty() {
            device.free_descriptor_sets(self.pool, &self.sets).ok();
        }
        if self.pool != vk::DescriptorPool::null() {
            device.destroy_descriptor_pool(self.pool, None);
        }
        if self.layout != vk::DescriptorSetLayout::null() {
            device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}

pub type MaterialId = u32;

#[derive(Clone, Debug)]
pub struct Material {
    pub id: MaterialId,
    pub name: String,
    pub descriptor_set: vk::DescriptorSet,
    pub textures: Vec<RRImage>,
    pub uniform_buffer: vk::Buffer,
    pub uniform_buffer_memory: vk::DeviceMemory,
    pub properties: MaterialUBO,
}

#[derive(Clone, Debug, Default)]
pub struct MaterialManager {
    pub layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
    pub materials: HashMap<MaterialId, Material>,
    next_id: MaterialId,
}

impl MaterialManager {
    pub unsafe fn new(rrdevice: &RRDevice, max_materials: u32) -> anyhow::Result<Self> {
        let layout = Self::create_layout(rrdevice)?;
        let pool = Self::create_pool(rrdevice, max_materials)?;

        Ok(Self {
            layout,
            pool,
            materials: HashMap::new(),
            next_id: 0,
        })
    }

    unsafe fn create_layout(rrdevice: &RRDevice) -> anyhow::Result<vk::DescriptorSetLayout> {
        let sampler_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        let ubo_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        let bindings = &[sampler_binding, ubo_binding];
        let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);

        Ok(rrdevice.device.create_descriptor_set_layout(&info, None)?)
    }

    unsafe fn create_pool(rrdevice: &RRDevice, max_materials: u32) -> anyhow::Result<vk::DescriptorPool> {
        let sampler_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(max_materials);

        let ubo_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(max_materials);

        let pool_sizes = &[sampler_size, ubo_size];
        let info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(pool_sizes)
            .max_sets(max_materials)
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);

        Ok(rrdevice.device.create_descriptor_pool(&info, None)?)
    }

    pub unsafe fn create_material(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        name: &str,
        texture: RRImage,
        properties: MaterialUBO,
    ) -> anyhow::Result<MaterialId> {
        self.create_material_with_texture(
            instance,
            rrdevice,
            name,
            texture.image_view,
            texture.sampler,
            properties,
        )
    }

    pub unsafe fn create_material_with_texture(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        name: &str,
        image_view: vk::ImageView,
        sampler: vk::Sampler,
        properties: MaterialUBO,
    ) -> anyhow::Result<MaterialId> {
        let layouts = &[self.layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.pool)
            .set_layouts(layouts);

        let descriptor_sets = rrdevice.device.allocate_descriptor_sets(&alloc_info)?;
        let descriptor_set = descriptor_sets[0];

        let (uniform_buffer, uniform_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            size_of::<MaterialUBO>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let memory = rrdevice.device.map_memory(
            uniform_buffer_memory,
            0,
            size_of::<MaterialUBO>() as u64,
            vk::MemoryMapFlags::empty(),
        )?;
        memcpy(&properties, memory.cast(), 1);
        rrdevice.device.unmap_memory(uniform_buffer_memory);

        let image_info = vk::DescriptorImageInfo::builder()
            .sampler(sampler)
            .image_view(image_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(uniform_buffer)
            .offset(0)
            .range(size_of::<MaterialUBO>() as u64);

        let image_infos = &[image_info];
        let buffer_infos = &[buffer_info];

        let sampler_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(image_infos);

        let ubo_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(1)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(buffer_infos);

        rrdevice.device.update_descriptor_sets(&[sampler_write, ubo_write], &[] as &[vk::CopyDescriptorSet]);

        let id = self.next_id;
        self.next_id += 1;

        let material = Material {
            id,
            name: name.to_string(),
            descriptor_set,
            textures: vec![],
            uniform_buffer,
            uniform_buffer_memory,
            properties,
        };

        self.materials.insert(id, material);
        Ok(id)
    }

    pub fn get(&self, id: MaterialId) -> Option<&Material> {
        self.materials.get(&id)
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        for material in self.materials.values() {
            device.destroy_buffer(material.uniform_buffer, None);
            device.free_memory(material.uniform_buffer_memory, None);
        }

        if self.pool != vk::DescriptorPool::null() {
            device.destroy_descriptor_pool(self.pool, None);
        }
        if self.layout != vk::DescriptorSetLayout::null() {
            device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}

pub type ObjectId = u32;

#[derive(Clone, Debug, Default)]
pub struct ObjectDescriptorSet {
    pub layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
    pub sets: Vec<vk::DescriptorSet>,
    pub buffers: Vec<vk::Buffer>,
    pub buffer_memories: Vec<vk::DeviceMemory>,
    pub max_objects: usize,
}

impl ObjectDescriptorSet {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        swapchain_image_count: usize,
        max_objects: usize,
    ) -> anyhow::Result<Self> {
        let layout = Self::create_layout(rrdevice)?;
        let total_sets = swapchain_image_count * max_objects;
        let pool = Self::create_pool(rrdevice, total_sets)?;
        let sets = Self::allocate_sets(rrdevice, layout, pool, total_sets)?;

        let mut buffers = Vec::with_capacity(total_sets);
        let mut buffer_memories = Vec::with_capacity(total_sets);

        for _ in 0..total_sets {
            let (buffer, memory) = create_buffer(
                instance,
                rrdevice,
                size_of::<ObjectUBO>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            buffers.push(buffer);
            buffer_memories.push(memory);
        }

        let mut object_set = Self {
            layout,
            pool,
            sets,
            buffers,
            buffer_memories,
            max_objects,
        };
        object_set.write_descriptor_sets(rrdevice, swapchain_image_count);

        Ok(object_set)
    }

    unsafe fn create_layout(rrdevice: &RRDevice) -> anyhow::Result<vk::DescriptorSetLayout> {
        let ubo_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);

        let bindings = &[ubo_binding];
        let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);

        Ok(rrdevice.device.create_descriptor_set_layout(&info, None)?)
    }

    unsafe fn create_pool(rrdevice: &RRDevice, count: usize) -> anyhow::Result<vk::DescriptorPool> {
        let pool_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(count as u32);

        let pool_sizes = &[pool_size];
        let info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(pool_sizes)
            .max_sets(count as u32)
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);

        Ok(rrdevice.device.create_descriptor_pool(&info, None)?)
    }

    unsafe fn allocate_sets(
        rrdevice: &RRDevice,
        layout: vk::DescriptorSetLayout,
        pool: vk::DescriptorPool,
        count: usize,
    ) -> anyhow::Result<Vec<vk::DescriptorSet>> {
        let layouts = vec![layout; count];
        let info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool)
            .set_layouts(&layouts);

        Ok(rrdevice.device.allocate_descriptor_sets(&info)?)
    }

    unsafe fn write_descriptor_sets(&mut self, rrdevice: &RRDevice, swapchain_image_count: usize) {
        for (i, &set) in self.sets.iter().enumerate() {
            let buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.buffers[i])
                .offset(0)
                .range(size_of::<ObjectUBO>() as u64);

            let buffer_infos = &[buffer_info];
            let write = vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(buffer_infos);

            rrdevice.device.update_descriptor_sets(&[write], &[] as &[vk::CopyDescriptorSet]);
        }
    }

    pub fn get_set_index(&self, image_index: usize, object_index: usize) -> usize {
        image_index * self.max_objects + object_index
    }

    pub unsafe fn update(
        &self,
        rrdevice: &RRDevice,
        image_index: usize,
        object_index: usize,
        ubo: &ObjectUBO,
    ) -> anyhow::Result<()> {
        let idx = self.get_set_index(image_index, object_index);
        let memory = rrdevice.device.map_memory(
            self.buffer_memories[idx],
            0,
            size_of::<ObjectUBO>() as u64,
            vk::MemoryMapFlags::empty(),
        )?;
        memcpy(ubo, memory.cast(), 1);
        rrdevice.device.unmap_memory(self.buffer_memories[idx]);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        for &buffer in &self.buffers {
            device.destroy_buffer(buffer, None);
        }
        for &memory in &self.buffer_memories {
            device.free_memory(memory, None);
        }

        if !self.sets.is_empty() {
            device.free_descriptor_sets(self.pool, &self.sets).ok();
        }
        if self.pool != vk::DescriptorPool::null() {
            device.destroy_descriptor_pool(self.pool, None);
        }
        if self.layout != vk::DescriptorSetLayout::null() {
            device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RenderResources {
    pub frame_set: FrameDescriptorSet,
    pub materials: MaterialManager,
    pub objects: ObjectDescriptorSet,
    pub meshes: Vec<Mesh>,
    pub mesh_material_ids: Vec<MaterialId>,
}

impl RenderResources {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        swapchain_image_count: usize,
        max_materials: u32,
        max_objects: usize,
    ) -> anyhow::Result<Self> {
        let frame_set = FrameDescriptorSet::new(instance, rrdevice, swapchain_image_count)?;
        let materials = MaterialManager::new(rrdevice, max_materials)?;
        let objects = ObjectDescriptorSet::new(instance, rrdevice, swapchain_image_count, max_objects)?;

        Ok(Self {
            frame_set,
            materials,
            objects,
            meshes: Vec::new(),
            mesh_material_ids: Vec::new(),
        })
    }

    pub fn get_material_id(&self, mesh_index: usize) -> Option<MaterialId> {
        self.mesh_material_ids.get(mesh_index).copied()
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    pub fn get_layouts(&self) -> [vk::DescriptorSetLayout; 3] {
        [
            self.frame_set.layout,
            self.materials.layout,
            self.objects.layout,
        ]
    }

    pub fn get_layouts_without_material(&self) -> [vk::DescriptorSetLayout; 2] {
        [
            self.frame_set.layout,
            self.objects.layout,
        ]
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        for mesh in &mut self.meshes {
            mesh.destroy(rrdevice);
        }
        self.meshes.clear();
        self.mesh_material_ids.clear();

        self.frame_set.destroy(&rrdevice.device);
        self.materials.destroy(&rrdevice.device);
        self.objects.destroy(&rrdevice.device);
    }

    pub unsafe fn clear_meshes(&mut self, rrdevice: &RRDevice) {
        for mesh in &mut self.meshes {
            mesh.destroy(rrdevice);
        }
        self.meshes.clear();
        self.mesh_material_ids.clear();
    }
}
