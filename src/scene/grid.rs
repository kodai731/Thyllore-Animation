use crate::ecs::resource::PipelineId;
use crate::render::{IndexBufferHandle, VertexBufferHandle};
use crate::vulkanr::data::Vertex;

#[derive(Clone, Debug, Default)]
pub struct GridData {
    pub pipeline_id: Option<PipelineId>,
    pub vertex_buffer_handle: VertexBufferHandle,
    pub index_buffer_handle: IndexBufferHandle,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub scale: f32,
    pub object_index: usize,
}
