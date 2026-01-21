use cgmath::{Matrix4, SquareMatrix};

use crate::ecs::resource::PipelineId;
use crate::render::{IndexBufferHandle, MeshId, VertexBufferHandle};

#[derive(Clone, Copy, Debug, Default)]
pub struct ObjectIndex(pub usize);

impl ObjectIndex {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn get(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MeshHandle {
    pub mesh_id: MeshId,
}

impl MeshHandle {
    pub fn new(mesh_id: MeshId) -> Self {
        Self { mesh_id }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SkeletonHandle {
    pub skeleton_id: usize,
}

#[derive(Clone, Debug)]
pub struct RenderData {
    pub object_index: usize,
    pub pipeline_id: Option<PipelineId>,
    pub vertex_buffer_handle: VertexBufferHandle,
    pub index_buffer_handle: IndexBufferHandle,
    pub index_count: u32,
    pub model_matrix: Matrix4<f32>,
}

impl Default for RenderData {
    fn default() -> Self {
        Self {
            object_index: 0,
            pipeline_id: None,
            vertex_buffer_handle: Default::default(),
            index_buffer_handle: Default::default(),
            index_count: 0,
            model_matrix: Matrix4::identity(),
        }
    }
}

impl RenderData {
    pub fn new(
        object_index: usize,
        pipeline_id: Option<PipelineId>,
        vertex_buffer_handle: VertexBufferHandle,
        index_buffer_handle: IndexBufferHandle,
        index_count: u32,
    ) -> Self {
        Self {
            object_index,
            pipeline_id,
            vertex_buffer_handle,
            index_buffer_handle,
            index_count,
            model_matrix: Matrix4::identity(),
        }
    }

    pub fn with_model_matrix(mut self, model_matrix: Matrix4<f32>) -> Self {
        self.model_matrix = model_matrix;
        self
    }
}
