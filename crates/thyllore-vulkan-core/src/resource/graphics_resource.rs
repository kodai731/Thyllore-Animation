use std::ffi::c_void;
use std::mem::size_of;
use std::rc::Rc;

use crate::command::RRCommandPool;
use crate::core::device::RRDevice;
use crate::data::{Vertex, VertexData};
use crate::descriptor::ReflectedSetLayout;
use crate::resource::buffer::{RRIndexBuffer, RRVertexBuffer};
use crate::resource::image::{
    create_image_view, create_texture_image_pixel, create_texture_sampler,
};
use crate::resource::GpuResource;
use crate::vulkan::*;
use cgmath::{Matrix4, SquareMatrix, Vector3, Vector4};
use thyllore_model_core::{SkeletonId, SkinData};
use thyllore_render_core::{MaterialUBO, ObjectUBO};

pub use crate::descriptor::{
    FrameDescriptorSet, Material, MaterialId, MaterialManager, ObjectDescriptorSet, ObjectId,
};
pub use crate::resource::MeshBuffer;

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

pub struct TexturePixels<'a> {
    pub rgba: &'a [u8],
    pub width: u32,
    pub height: u32,
}

pub struct MeshSource<'a> {
    pub vertex_data: &'a VertexData,
    pub base_vertices: &'a [Vertex],
    pub skin_data: Option<&'a SkinData>,
    pub skeleton_id: Option<SkeletonId>,
    pub node_index: Option<usize>,
    pub texture: TexturePixels<'a>,
    pub base_color_factor: [f32; 4],
}

#[derive(Clone, Debug, Default)]
pub struct GraphicsResources {
    pub frame_set: FrameDescriptorSet,
    pub materials: MaterialManager,
    pub objects: ObjectDescriptorSet,
    pub meshes: Vec<MeshBuffer>,
    pub mesh_material_ids: Vec<MaterialId>,
}

impl GraphicsResources {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        swapchain_image_count: usize,
        max_objects: usize,
    ) -> anyhow::Result<Self> {
        let frame_set = FrameDescriptorSet::new(instance, rrdevice, swapchain_image_count)?;
        let materials = MaterialManager::new(rrdevice)?;
        let objects =
            ObjectDescriptorSet::new(instance, rrdevice, swapchain_image_count, max_objects)?;

        Ok(Self {
            frame_set,
            materials,
            objects,
            meshes: Vec::new(),
            mesh_material_ids: Vec::new(),
        })
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

    pub unsafe fn ensure_object_capacity(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        swapchain_image_count: usize,
        additional_meshes: usize,
    ) -> anyhow::Result<()> {
        let required_objects = self.objects.get_next_slot() + additional_meshes;

        self.objects
            .ensure_capacity(instance, rrdevice, swapchain_image_count, required_objects)
    }

    pub unsafe fn push_mesh(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &Rc<RRCommandPool>,
        source: &MeshSource,
    ) -> anyhow::Result<usize> {
        let mesh_index = self.meshes.len();
        let mesh = self.create_mesh_buffer(instance, rrdevice, command_pool, source, mesh_index)?;
        let material_id = self.create_material(instance, rrdevice, &mesh, mesh_index, source)?;

        self.meshes.push(mesh);
        self.mesh_material_ids.push(material_id);
        Ok(mesh_index)
    }

    unsafe fn create_mesh_buffer(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &Rc<RRCommandPool>,
        source: &MeshSource,
        mesh_index: usize,
    ) -> anyhow::Result<MeshBuffer> {
        let mut mesh = MeshBuffer::default();

        let pixels = source.texture.rgba.to_vec();
        (mesh.image, mesh.image_memory, mesh.mip_level) = create_texture_image_pixel(
            instance,
            rrdevice,
            command_pool,
            &pixels,
            source.texture.width,
            source.texture.height,
        )?;
        mesh.image_view = create_image_view(
            rrdevice,
            mesh.image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageAspectFlags::COLOR,
            mesh.mip_level,
        )?;
        mesh.sampler = create_texture_sampler(rrdevice, mesh.mip_level)?;

        mesh.vertex_data = source.vertex_data.clone();
        mesh.skin_data = source.skin_data.cloned();
        mesh.skeleton_id = source.skeleton_id;
        mesh.node_index = source.node_index;
        mesh.base_vertices = source.base_vertices.to_vec();
        mesh.base_colors = Some(
            mesh.vertex_data
                .vertices
                .iter()
                .map(|v| Vector4::new(v.color.x, v.color.y, v.color.z, v.color.w))
                .collect(),
        );

        mesh.vertex_buffer = RRVertexBuffer::new(
            instance,
            rrdevice,
            command_pool,
            (size_of::<Vertex>() * mesh.vertex_data.vertices.len()) as vk::DeviceSize,
            mesh.vertex_data.vertices.as_ptr() as *const c_void,
            mesh.vertex_data.vertices.len(),
        )?;
        mesh.index_buffer = RRIndexBuffer::new(
            instance,
            rrdevice,
            command_pool,
            (size_of::<u32>() * mesh.vertex_data.indices.len()) as u64,
            mesh.vertex_data.indices.as_ptr() as *const c_void,
            mesh.vertex_data.indices.len(),
        )?;

        mesh.object_index = self.objects.allocate_slot();
        log!(
            "Allocated object_index {} for mesh {}",
            mesh.object_index,
            mesh_index
        );

        Ok(mesh)
    }

    unsafe fn create_material(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        mesh: &MeshBuffer,
        mesh_index: usize,
        source: &MeshSource,
    ) -> anyhow::Result<MaterialId> {
        let [r, g, b, a] = source.base_color_factor;
        let properties = MaterialUBO {
            base_color: Vector4::new(r, g, b, a),
            ..MaterialUBO::default()
        };

        let material_id = self.materials.create_material_with_texture(
            instance,
            rrdevice,
            &format!("material_{}", mesh_index),
            mesh.image_view,
            mesh.sampler,
            properties,
        )?;

        log!("Created material {} for mesh {}", material_id, mesh_index);
        Ok(material_id)
    }

    pub unsafe fn upload_mesh_vertices(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &RRCommandPool,
        mesh_index: usize,
    ) -> anyhow::Result<()> {
        match self.meshes.get_mut(mesh_index) {
            Some(mesh) => mesh.upload_vertices(instance, rrdevice, command_pool),
            None => Ok(()),
        }
    }

    pub fn get_material_id(&self, mesh_index: usize) -> Option<MaterialId> {
        self.mesh_material_ids.get(mesh_index).copied()
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    pub fn get_layouts(&self) -> [&ReflectedSetLayout; 3] {
        [
            &self.frame_set.layout,
            &self.materials.layout,
            &self.objects.layout,
        ]
    }

    pub fn get_layouts_without_material(&self) -> [&ReflectedSetLayout; 2] {
        [&self.frame_set.layout, &self.objects.layout]
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

        log!("Model bounds: min=({:.2}, {:.2}, {:.2}), max=({:.2}, {:.2}, {:.2}), center=({:.2}, {:.2}, {:.2})",
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
    ) -> crate::pipeline::PipelineKey {
        crate::pipeline::PipelineKey::new(
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

impl GpuResource for GraphicsResources {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }
}
