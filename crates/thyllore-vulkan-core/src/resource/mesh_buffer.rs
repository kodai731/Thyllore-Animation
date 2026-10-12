use std::ffi::c_void;
use std::mem::size_of;

use anyhow::Result;
use cgmath::Vector4;
use vulkanalia::prelude::v1_0::*;

use crate::command::RRCommandPool;
use crate::core::device::RRDevice;
use crate::data::{Vertex, VertexData};
use crate::resource::allocation::TextureImage;
use crate::resource::buffer::{RRIndexBuffer, RRVertexBuffer};
use crate::resource::gpu_resource::GpuResource;
use crate::vulkan::Instance;
use thyllore_model_core::{MeshMorph, SkeletonId, SkinData};

#[must_use]
#[derive(Debug, GpuResource)]
pub struct MeshBuffer {
    pub texture: TextureImage,
    pub vertex_buffer: RRVertexBuffer,
    pub index_buffer: RRIndexBuffer,
    #[gpu_resource(skip)]
    pub vertex_data: VertexData,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    #[gpu_resource(skip)]
    pub render_to_gbuffer: bool,
    #[gpu_resource(skip)]
    pub object_index: usize,
    #[gpu_resource(skip)]
    pub skin_data: Option<SkinData>,
    #[gpu_resource(skip)]
    pub skeleton_id: Option<SkeletonId>,
    #[gpu_resource(skip)]
    pub node_index: Option<usize>,
    #[gpu_resource(skip)]
    pub base_vertices: Vec<Vertex>,
    #[gpu_resource(skip)]
    pub base_colors: Option<Vec<Vector4<f32>>>,
    #[gpu_resource(skip)]
    pub morph: MeshMorph,
}

impl Default for MeshBuffer {
    fn default() -> Self {
        Self {
            texture: TextureImage::default(),
            vertex_buffer: RRVertexBuffer::default(),
            index_buffer: RRIndexBuffer::default(),
            vertex_data: VertexData::default(),
            image_view: vk::ImageView::null(),
            sampler: vk::Sampler::null(),
            render_to_gbuffer: true,
            object_index: 0,
            skin_data: None,
            skeleton_id: None,
            node_index: None,
            base_vertices: Vec::new(),
            base_colors: None,
            morph: MeshMorph::default(),
        }
    }
}

impl MeshBuffer {
    pub unsafe fn upload_vertices(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &RRCommandPool,
    ) -> Result<()> {
        let vertices = &self.vertex_data.vertices;
        self.vertex_buffer.update(
            instance,
            rrdevice,
            command_pool,
            (size_of::<Vertex>() * vertices.len()) as vk::DeviceSize,
            vertices.as_ptr() as *const c_void,
            vertices.len(),
        )
    }
}
