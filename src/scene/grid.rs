use cgmath::Matrix4;

use crate::ecs::RenderData;
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

impl GridData {
    pub fn render_data(&self) -> RenderData {
        RenderData {
            object_index: self.object_index,
            pipeline: self.pipeline.clone(),
            vertex_buffer: self.vertex_buffer.buffer,
            index_buffer: self.index_buffer.buffer,
            index_count: self.index_buffer.indices,
            model_matrix: Matrix4::from_scale(self.scale),
        }
    }
}
