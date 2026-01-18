use cgmath::{Matrix4, SquareMatrix};
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::PipelineId;

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

#[derive(Clone, Debug)]
pub struct RenderData {
    pub object_index: usize,
    pub pipeline_id: Option<PipelineId>,
    pub vertex_buffer: vk::Buffer,
    pub index_buffer: vk::Buffer,
    pub index_count: u32,
    pub model_matrix: Matrix4<f32>,
}

impl Default for RenderData {
    fn default() -> Self {
        Self {
            object_index: 0,
            pipeline_id: None,
            vertex_buffer: vk::Buffer::null(),
            index_buffer: vk::Buffer::null(),
            index_count: 0,
            model_matrix: Matrix4::identity(),
        }
    }
}

impl RenderData {
    pub fn new(
        object_index: usize,
        pipeline_id: Option<PipelineId>,
        vertex_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        index_count: u32,
    ) -> Self {
        Self {
            object_index,
            pipeline_id,
            vertex_buffer,
            index_buffer,
            index_count,
            model_matrix: Matrix4::identity(),
        }
    }

    pub fn with_model_matrix(mut self, model_matrix: Matrix4<f32>) -> Self {
        self.model_matrix = model_matrix;
        self
    }
}
