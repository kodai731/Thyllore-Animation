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
