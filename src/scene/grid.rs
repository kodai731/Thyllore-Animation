use crate::vulkanr::buffer::{RRIndexBuffer, RRVertexBuffer};
use crate::vulkanr::data::Vertex;
use crate::vulkanr::descriptor::RRDescriptorSet;
use crate::vulkanr::pipeline::RRPipeline;

#[derive(Clone, Debug, Default)]
pub struct GridData {
    pub pipeline: RRPipeline,
    pub descriptor_set: RRDescriptorSet,
    pub vertex_buffer: RRVertexBuffer,
    pub index_buffer: RRIndexBuffer,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub scale: f32,
}
