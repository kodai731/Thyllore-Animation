use crate::animation::{AnimationSystem, MorphAnimationSystem, SkeletonId, SkinData};
use crate::vulkanr::buffer::{RRIndexBuffer, RRVertexBuffer};
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::core::device::RRDevice;
use crate::vulkanr::data::{Vertex, VertexData};
use crate::vulkanr::raytracing::acceleration::RRAccelerationStructure;
use crate::vulkanr::resource::buffer::create_buffer;
use crate::vulkanr::resource::image::RRImage;
use crate::vulkanr::vulkan::*;
use cgmath::{Matrix4, SquareMatrix, Vector3, Vector4};
use std::collections::HashMap;
use std::ffi::c_void;
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
pub struct NodeData {
    pub index: usize,
    pub name: String,
    pub parent_index: Option<usize>,
    pub local_transform: Matrix4<f32>,
    pub global_transform: Matrix4<f32>,
}

impl Default for NodeData {
    fn default() -> Self {
        Self {
            index: 0,
            name: String::new(),
            parent_index: None,
            local_transform: Matrix4::identity(),
            global_transform: Matrix4::identity(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MeshBuffer {
    pub vertex_buffer: RRVertexBuffer,
    pub index_buffer: RRIndexBuffer,
    pub vertex_data: VertexData,
    pub image: vk::Image,
    pub image_memory: vk::DeviceMemory,
    pub mip_level: u32,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub render_to_gbuffer: bool,
    pub object_index: usize,
    pub skin_data: Option<SkinData>,
    pub skeleton_id: Option<SkeletonId>,
    pub node_index: Option<usize>,
    pub base_vertices: Vec<Vertex>,
}

impl Default for MeshBuffer {
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
            object_index: 0,
            skin_data: None,
            skeleton_id: None,
            node_index: None,
            base_vertices: Vec::new(),
        }
    }
}

impl MeshBuffer {
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
            rrdevice
                .device
                .destroy_buffer(self.vertex_buffer.buffer, None);
            self.vertex_buffer.buffer = vk::Buffer::null();
        }
        if self.vertex_buffer.buffer_memory != vk::DeviceMemory::null() {
            rrdevice
                .device
                .free_memory(self.vertex_buffer.buffer_memory, None);
            self.vertex_buffer.buffer_memory = vk::DeviceMemory::null();
        }
        if self.index_buffer.buffer != vk::Buffer::null() {
            rrdevice
                .device
                .destroy_buffer(self.index_buffer.buffer, None);
            self.index_buffer.buffer = vk::Buffer::null();
        }
        if self.index_buffer.buffer_memory != vk::DeviceMemory::null() {
            rrdevice
                .device
                .free_memory(self.index_buffer.buffer_memory, None);
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

            rrdevice
                .device
                .update_descriptor_sets(&[write], &[] as &[vk::CopyDescriptorSet]);
        }
    }

    pub unsafe fn update(
        &self,
        rrdevice: &RRDevice,
        image_index: usize,
        ubo: &FrameUBO,
    ) -> anyhow::Result<()> {
        let memory = rrdevice.device.map_memory(
            self.buffer_memories[image_index],
            0,
            size_of::<FrameUBO>() as u64,
            vk::MemoryMapFlags::empty(),
        )?;
        memcpy(ubo, memory.cast(), 1);
        rrdevice
            .device
            .unmap_memory(self.buffer_memories[image_index]);
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

    unsafe fn create_pool(
        rrdevice: &RRDevice,
        max_materials: u32,
    ) -> anyhow::Result<vk::DescriptorPool> {
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

        rrdevice
            .device
            .update_descriptor_sets(&[sampler_write, ubo_write], &[] as &[vk::CopyDescriptorSet]);

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

    pub unsafe fn clear_materials(&mut self, device: &vulkanalia::Device) {
        for material in self.materials.values() {
            device.destroy_buffer(material.uniform_buffer, None);
            device.free_memory(material.uniform_buffer_memory, None);
        }

        if self.pool != vk::DescriptorPool::null() {
            device
                .reset_descriptor_pool(self.pool, vk::DescriptorPoolResetFlags::empty())
                .ok();
        }

        self.materials.clear();
        self.next_id = 0;
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
    next_slot: usize,
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
            next_slot: 0,
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

            rrdevice
                .device
                .update_descriptor_sets(&[write], &[] as &[vk::CopyDescriptorSet]);
        }
    }

    pub fn get_set_index(&self, image_index: usize, object_index: usize) -> usize {
        image_index * self.max_objects + object_index
    }

    pub fn allocate_slot(&mut self) -> usize {
        let slot = self.next_slot;
        self.next_slot += 1;
        slot
    }

    pub fn get_next_slot(&self) -> usize {
        self.next_slot
    }

    pub fn reset_to(&mut self, slot: usize) {
        self.next_slot = slot;
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
pub struct GraphicsResources {
    pub frame_set: FrameDescriptorSet,
    pub materials: MaterialManager,
    pub objects: ObjectDescriptorSet,
    pub meshes: Vec<MeshBuffer>,
    pub mesh_material_ids: Vec<MaterialId>,
    pub nodes: Vec<NodeData>,
    pub animation: AnimationSystem,
    pub morph_animation: MorphAnimationSystem,
    pub has_skinned_meshes: bool,
    pub node_animation_scale: f32,
}

impl GraphicsResources {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        swapchain_image_count: usize,
        max_materials: u32,
        max_objects: usize,
    ) -> anyhow::Result<Self> {
        let frame_set = FrameDescriptorSet::new(instance, rrdevice, swapchain_image_count)?;
        let materials = MaterialManager::new(rrdevice, max_materials)?;
        let objects =
            ObjectDescriptorSet::new(instance, rrdevice, swapchain_image_count, max_objects)?;

        Ok(Self {
            frame_set,
            materials,
            objects,
            meshes: Vec::new(),
            mesh_material_ids: Vec::new(),
            nodes: Vec::new(),
            animation: AnimationSystem::new(),
            morph_animation: MorphAnimationSystem::new(),
            has_skinned_meshes: false,
            node_animation_scale: 1.0,
        })
    }

    pub unsafe fn update_skinned_vertex_buffers(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &RRCommandPool,
    ) -> anyhow::Result<()> {
        for mesh_idx in 0..self.meshes.len() {
            let (skin_data, skeleton_id) = {
                let mesh = &self.meshes[mesh_idx];
                (mesh.skin_data.clone(), mesh.skeleton_id)
            };

            let Some(skin_data) = skin_data else {
                continue;
            };
            let Some(skeleton_id) = skeleton_id else {
                continue;
            };
            let Some(skeleton) = self.animation.get_skeleton(skeleton_id) else {
                continue;
            };

            let vertex_count = skin_data.base_positions.len();
            let mut skinned_positions = vec![Vector3::new(0.0, 0.0, 0.0); vertex_count];
            let mut skinned_normals = vec![Vector3::new(0.0, 1.0, 0.0); vertex_count];

            skin_data.apply_skinning(skeleton, &mut skinned_positions, &mut skinned_normals);

            let mesh = &mut self.meshes[mesh_idx];
            for (i, pos) in skinned_positions.iter().enumerate() {
                if i < mesh.vertex_data.vertices.len() {
                    mesh.vertex_data.vertices[i].pos.x = pos.x;
                    mesh.vertex_data.vertices[i].pos.y = pos.y;
                    mesh.vertex_data.vertices[i].pos.z = pos.z;
                }
            }
            for (i, normal) in skinned_normals.iter().enumerate() {
                if i < mesh.vertex_data.vertices.len() {
                    mesh.vertex_data.vertices[i].normal.x = normal.x;
                    mesh.vertex_data.vertices[i].normal.y = normal.y;
                    mesh.vertex_data.vertices[i].normal.z = normal.z;
                }
            }

            if let Err(e) = mesh.vertex_buffer.update(
                instance,
                rrdevice,
                command_pool,
                (size_of::<Vertex>() * mesh.vertex_data.vertices.len()) as vk::DeviceSize,
                mesh.vertex_data.vertices.as_ptr() as *const c_void,
                mesh.vertex_data.vertices.len(),
            ) {
                crate::log!(
                    "Failed to update skinned vertex buffer for mesh {}: {}",
                    mesh_idx,
                    e
                );
            }
        }

        Ok(())
    }

    pub unsafe fn update_acceleration_structure(
        &self,
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &RRCommandPool,
        acceleration_structure: &Option<RRAccelerationStructure>,
    ) -> anyhow::Result<()> {
        let Some(ref accel_struct) = acceleration_structure else {
            return Ok(());
        };

        let vertex_buffers: Vec<_> = self
            .meshes
            .iter()
            .filter(|mesh| mesh.vertex_buffer.buffer != vk::Buffer::null())
            .map(|mesh| {
                (
                    &mesh.vertex_buffer.buffer,
                    mesh.vertex_data.vertices.len() as u32,
                    size_of::<Vertex>() as u32,
                    &mesh.index_buffer.buffer,
                    mesh.vertex_data.indices.len() as u32,
                )
            })
            .collect();

        if !vertex_buffers.is_empty() {
            accel_struct.update_all(instance, rrdevice, command_pool, &vertex_buffers)?;
        }

        Ok(())
    }

    pub unsafe fn update_objects(
        &self,
        rrdevice: &RRDevice,
        image_index: usize,
        model: Matrix4<f32>,
    ) -> anyhow::Result<()> {
        for mesh in &self.meshes {
            let object_ubo = ObjectUBO { model };
            self.objects
                .update(rrdevice, image_index, mesh.object_index, &object_ubo)?;
        }
        Ok(())
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
        [self.frame_set.layout, self.objects.layout]
    }

    pub fn calculate_model_bounds(&self) -> Option<(Vector3<f32>, Vector3<f32>, Vector3<f32>)> {
        if self.meshes.is_empty() {
            return None;
        }

        let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vector3::new(f32::MIN, f32::MIN, f32::MIN);
        let mut has_vertices = false;

        for mesh in &self.meshes {
            for vertex in &mesh.vertex_data.vertices {
                has_vertices = true;
                min.x = min.x.min(vertex.pos.x);
                min.y = min.y.min(vertex.pos.y);
                min.z = min.z.min(vertex.pos.z);
                max.x = max.x.max(vertex.pos.x);
                max.y = max.y.max(vertex.pos.y);
                max.z = max.z.max(vertex.pos.z);
            }
        }

        if !has_vertices {
            return None;
        }

        let center = Vector3::new(
            (min.x + max.x) * 0.5,
            (min.y + max.y) * 0.5,
            (min.z + max.z) * 0.5,
        );

        crate::log!("Model bounds: min=({:.2}, {:.2}, {:.2}), max=({:.2}, {:.2}, {:.2}), center=({:.2}, {:.2}, {:.2})",
            min.x, min.y, min.z, max.x, max.y, max.z, center.x, center.y, center.z);

        Some((min, max, center))
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

    fn compute_node_global_transforms(&mut self) {
        static mut TRANSFORM_LOG: u32 = 0;

        if self.nodes.is_empty() {
            return;
        }

        let mut matched_count = 0;
        for skeleton in &self.animation.skeletons {
            for bone in &skeleton.bones {
                if let Some(node) = self.nodes.iter_mut().find(|n| n.name == bone.name) {
                    unsafe {
                        if TRANSFORM_LOG < 1 {
                            let orig = node.local_transform;
                            let anim = bone.local_transform;
                            let orig_scale = (
                                (orig[0][0] * orig[0][0]
                                    + orig[0][1] * orig[0][1]
                                    + orig[0][2] * orig[0][2])
                                    .sqrt(),
                                (orig[1][0] * orig[1][0]
                                    + orig[1][1] * orig[1][1]
                                    + orig[1][2] * orig[1][2])
                                    .sqrt(),
                                (orig[2][0] * orig[2][0]
                                    + orig[2][1] * orig[2][1]
                                    + orig[2][2] * orig[2][2])
                                    .sqrt(),
                            );
                            let anim_scale = (
                                (anim[0][0] * anim[0][0]
                                    + anim[0][1] * anim[0][1]
                                    + anim[0][2] * anim[0][2])
                                    .sqrt(),
                                (anim[1][0] * anim[1][0]
                                    + anim[1][1] * anim[1][1]
                                    + anim[1][2] * anim[1][2])
                                    .sqrt(),
                                (anim[2][0] * anim[2][0]
                                    + anim[2][1] * anim[2][1]
                                    + anim[2][2] * anim[2][2])
                                    .sqrt(),
                            );
                            crate::log!(
                                "  bone '{}' node[{}]: orig_t=[{:.2},{:.2},{:.2}] anim_t=[{:.2},{:.2},{:.2}]",
                                bone.name, node.index,
                                orig[3][0], orig[3][1], orig[3][2],
                                anim[3][0], anim[3][1], anim[3][2]
                            );
                            crate::log!(
                                "    orig_s=[{:.2},{:.2},{:.2}] anim_s=[{:.2},{:.2},{:.2}]",
                                orig_scale.0,
                                orig_scale.1,
                                orig_scale.2,
                                anim_scale.0,
                                anim_scale.1,
                                anim_scale.2
                            );
                        }
                    }
                    node.local_transform = bone.local_transform;
                    matched_count += 1;
                }
            }
        }

        unsafe {
            if TRANSFORM_LOG < 1 {
                crate::log!(
                    "compute_node_global_transforms: {} bones matched to {} nodes",
                    matched_count,
                    self.nodes.len()
                );
                crate::log!("=== Node Hierarchy (with transforms) ===");
                for node in &self.nodes {
                    let parent_name = node
                        .parent_index
                        .and_then(|pi| self.nodes.iter().find(|pn| pn.index == pi))
                        .map(|pn| pn.name.as_str())
                        .unwrap_or("(root)");
                    let lt = node.local_transform;
                    let scale = (
                        (lt[0][0] * lt[0][0] + lt[0][1] * lt[0][1] + lt[0][2] * lt[0][2]).sqrt(),
                        (lt[1][0] * lt[1][0] + lt[1][1] * lt[1][1] + lt[1][2] * lt[1][2]).sqrt(),
                        (lt[2][0] * lt[2][0] + lt[2][1] * lt[2][1] + lt[2][2] * lt[2][2]).sqrt(),
                    );
                    if scale.0 > 1.01 || scale.1 > 1.01 || scale.2 > 1.01 {
                        crate::log!(
                            "  node[{}] '{}' SCALE=[{:.1},{:.1},{:.1}] parent='{}'",
                            node.index,
                            node.name,
                            scale.0,
                            scale.1,
                            scale.2,
                            parent_name
                        );
                    }
                }
                TRANSFORM_LOG += 1;
            }
        }

        let node_count = self.nodes.len();

        fn compute_global(
            nodes: &[NodeData],
            node_idx: usize,
            computed: &mut [bool],
            global_transforms: &mut [Matrix4<f32>],
        ) -> Matrix4<f32> {
            if computed[node_idx] {
                return global_transforms[node_idx];
            }

            let local = nodes[node_idx].local_transform;
            let global = if let Some(parent_idx) = nodes[node_idx].parent_index {
                if let Some(parent_array_idx) = nodes.iter().position(|n| n.index == parent_idx) {
                    let parent_global =
                        compute_global(nodes, parent_array_idx, computed, global_transforms);
                    parent_global * local
                } else {
                    local
                }
            } else {
                local
            };

            global_transforms[node_idx] = global;
            computed[node_idx] = true;
            global
        }

        let mut computed = vec![false; node_count];
        let mut global_transforms = vec![Matrix4::identity(); node_count];

        for i in 0..node_count {
            compute_global(&self.nodes, i, &mut computed, &mut global_transforms);
        }

        for (i, node) in self.nodes.iter_mut().enumerate() {
            node.global_transform = global_transforms[i];
        }
    }

    pub unsafe fn update_node_animation(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &RRCommandPool,
        acceleration_structure: &mut Option<RRAccelerationStructure>,
    ) -> anyhow::Result<Vec<usize>> {
        static mut LOG_COUNT: u32 = 0;

        self.compute_node_global_transforms();

        let mut updated_mesh_indices = Vec::new();
        let scale = self.node_animation_scale;

        for (mesh_idx, mesh) in self.meshes.iter_mut().enumerate() {
            if mesh.skin_data.is_some() || mesh.base_vertices.is_empty() {
                continue;
            }

            let Some(node_idx) = mesh.node_index else {
                continue;
            };

            let node_found = self.nodes.iter().find(|n| n.index == node_idx);
            if LOG_COUNT < 1 {
                if let Some(n) = &node_found {
                    let parent_name = n
                        .parent_index
                        .and_then(|pi| self.nodes.iter().find(|pn| pn.index == pi))
                        .map(|pn| pn.name.as_str())
                        .unwrap_or("(none)");
                    crate::log!(
                        "update_node_anim: mesh[{}] node='{}' idx={}, parent='{}' (idx={:?})",
                        mesh_idx,
                        n.name,
                        node_idx,
                        parent_name,
                        n.parent_index
                    );
                    crate::log!(
                        "  local: diag=[{:.2},{:.2},{:.2}] trans=[{:.2},{:.2},{:.2}]",
                        n.local_transform[0][0],
                        n.local_transform[1][1],
                        n.local_transform[2][2],
                        n.local_transform[3][0],
                        n.local_transform[3][1],
                        n.local_transform[3][2]
                    );
                    crate::log!(
                        "  global: diag=[{:.2},{:.2},{:.2}] trans=[{:.2},{:.2},{:.2}]",
                        n.global_transform[0][0],
                        n.global_transform[1][1],
                        n.global_transform[2][2],
                        n.global_transform[3][0],
                        n.global_transform[3][1],
                        n.global_transform[3][2]
                    );
                }
            }

            let Some(node) = node_found else {
                continue;
            };

            let transform = node.global_transform;

            if LOG_COUNT < 1 && mesh_idx == 2 {
                crate::log!("=== mesh[2] global_transform chain ===");
                let mut current_idx = Some(node_idx);
                while let Some(idx) = current_idx {
                    if let Some(n) = self.nodes.iter().find(|nn| nn.index == idx) {
                        let s = (
                            (n.global_transform[0][0].powi(2)
                                + n.global_transform[0][1].powi(2)
                                + n.global_transform[0][2].powi(2))
                            .sqrt(),
                            (n.global_transform[1][0].powi(2)
                                + n.global_transform[1][1].powi(2)
                                + n.global_transform[1][2].powi(2))
                            .sqrt(),
                            (n.global_transform[2][0].powi(2)
                                + n.global_transform[2][1].powi(2)
                                + n.global_transform[2][2].powi(2))
                            .sqrt(),
                        );
                        crate::log!(
                            "  {} (idx={}) global_scale=[{:.1},{:.1},{:.1}] trans=[{:.1},{:.1},{:.1}]",
                            n.name, n.index, s.0, s.1, s.2,
                            n.global_transform[3][0], n.global_transform[3][1], n.global_transform[3][2]
                        );
                        current_idx = n.parent_index;
                    } else {
                        break;
                    }
                }
            }

            if LOG_COUNT < 1 && !mesh.base_vertices.is_empty() {
                let base = &mesh.base_vertices[0];
                let orig = &mesh.vertex_data.vertices[0];
                crate::log!(
                    "  base_v[0]=({:.2},{:.2},{:.2}), orig_v[0]=({:.2},{:.2},{:.2})",
                    base.pos.x,
                    base.pos.y,
                    base.pos.z,
                    orig.pos.x,
                    orig.pos.y,
                    orig.pos.z
                );
            }

            for (i, v) in mesh.vertex_data.vertices.iter_mut().enumerate() {
                if i < mesh.base_vertices.len() {
                    let base = &mesh.base_vertices[i];
                    let pos = transform * Vector4::new(base.pos.x, base.pos.y, base.pos.z, 1.0);
                    v.pos.x = pos.x * scale;
                    v.pos.y = pos.y * scale;
                    v.pos.z = pos.z * scale;
                }
            }

            if LOG_COUNT < 1 && !mesh.vertex_data.vertices.is_empty() {
                let v = &mesh.vertex_data.vertices[0];
                crate::log!(
                    "  after transform: v[0]=({:.2},{:.2},{:.2})",
                    v.pos.x,
                    v.pos.y,
                    v.pos.z
                );
            }

            updated_mesh_indices.push(mesh_idx);
        }

        for mesh_idx in &updated_mesh_indices {
            let mesh = &mut self.meshes[*mesh_idx];
            let vertices = &mesh.vertex_data.vertices;

            mesh.vertex_buffer.update(
                instance,
                rrdevice,
                command_pool,
                (std::mem::size_of::<Vertex>() * vertices.len()) as vk::DeviceSize,
                vertices.as_ptr() as *const c_void,
                vertices.len(),
            )?;

            if let Some(ref mut accel_struct) = acceleration_structure {
                if *mesh_idx < accel_struct.blas_list.len() {
                    let blas = &accel_struct.blas_list[*mesh_idx];
                    RRAccelerationStructure::update_blas(
                        instance,
                        rrdevice,
                        command_pool,
                        blas,
                        &mesh.vertex_buffer.buffer,
                        mesh.vertex_data.vertices.len() as u32,
                        std::mem::size_of::<Vertex>() as u32,
                        &mesh.index_buffer.buffer,
                        mesh.vertex_data.indices.len() as u32,
                    )?;
                }
            }
        }

        if !updated_mesh_indices.is_empty() {
            if let Some(ref mut accel_struct) = acceleration_structure {
                let tlas = &accel_struct.tlas;
                RRAccelerationStructure::update_tlas(
                    instance,
                    rrdevice,
                    command_pool,
                    tlas,
                    &accel_struct.blas_list,
                )?;
            }
        }

        if LOG_COUNT < 1 {
            crate::log!(
                "update_node_anim: updated {} meshes",
                updated_mesh_indices.len()
            );
            LOG_COUNT += 1;
        }

        Ok(updated_mesh_indices)
    }

    pub fn create_pipeline_key(
        &self,
        vertex_shader: &str,
        fragment_shader: &str,
        topology: vk::PrimitiveTopology,
        polygon_mode: vk::PolygonMode,
        cull_mode: vk::CullModeFlags,
        depth_test_enable: bool,
        blend_enable: bool,
        render_pass: vk::RenderPass,
    ) -> crate::vulkanr::pipeline::PipelineKey {
        crate::vulkanr::pipeline::PipelineKey::new(
            vertex_shader,
            fragment_shader,
            topology,
            polygon_mode,
            cull_mode,
            depth_test_enable,
            depth_test_enable,
            blend_enable,
            vk::SampleCountFlags::_1,
            1,
            render_pass,
        )
    }
}
