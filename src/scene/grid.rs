use cgmath::Matrix4;
use vulkanalia::vk;

use crate::scene::components::{Renderable, RenderContext, Updatable, UpdateContext};
use crate::vulkanr::buffer::{RRIndexBuffer, RRVertexBuffer};
use crate::vulkanr::data::Vertex;
use crate::vulkanr::pipeline::RRPipeline;

#[derive(Clone, Debug, Default)]
pub struct GridData {
    pub pipeline: RRPipeline,
    pub vertex_buffer: RRVertexBuffer,
    pub index_buffer: RRIndexBuffer,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub scale: f32,
    pub object_index: usize,
}

impl Updatable for GridData {
    fn update(&mut self, _ctx: &UpdateContext) {}
}

impl Renderable for GridData {
    fn object_index(&self) -> usize {
        self.object_index
    }

    fn model_matrix(&self, _ctx: &RenderContext) -> Matrix4<f32> {
        Matrix4::from_scale(self.scale)
    }

    fn pipeline(&self) -> &RRPipeline {
        &self.pipeline
    }

    fn vertex_buffer(&self) -> vk::Buffer {
        self.vertex_buffer.buffer
    }

    fn index_buffer(&self) -> vk::Buffer {
        self.index_buffer.buffer
    }

    fn index_count(&self) -> u32 {
        self.index_buffer.indices
    }
}
