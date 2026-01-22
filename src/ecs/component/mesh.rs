use crate::ecs::resource::PipelineId;
use crate::render::{IndexBufferHandle, VertexBufferHandle};

use super::GizmoVertex;

#[derive(Clone, Debug, Default)]
pub struct LineMesh {
    pub pipeline_id: Option<PipelineId>,
    pub object_index: usize,
    pub vertices: Vec<GizmoVertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer_handle: VertexBufferHandle,
    pub index_buffer_handle: IndexBufferHandle,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MeshScale(pub f32);

impl MeshScale {
    pub fn new(scale: f32) -> Self {
        Self(scale)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}
