use cgmath::{Matrix4, SquareMatrix, Vector3};
use vulkanalia::prelude::v1_0::*;

use crate::vulkanr::pipeline::RRPipeline;

#[derive(Clone, Copy, Debug)]
pub struct CameraState {
    pub position: Vector3<f32>,
    pub direction: Vector3<f32>,
}

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
    pub pipeline: RRPipeline,
    pub vertex_buffer: vk::Buffer,
    pub index_buffer: vk::Buffer,
    pub index_count: u32,
    pub model_matrix: Matrix4<f32>,
}

impl Default for RenderData {
    fn default() -> Self {
        Self {
            object_index: 0,
            pipeline: RRPipeline::default(),
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
        pipeline: RRPipeline,
        vertex_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        index_count: u32,
    ) -> Self {
        Self {
            object_index,
            pipeline,
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
