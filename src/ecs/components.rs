use std::mem::size_of;

use cgmath::{Matrix4, SquareMatrix, Vector3};
use vulkanalia::prelude::v1_0::*;

use crate::vulkanr::data::Vertex;
use crate::vulkanr::pipeline::RRPipeline;

#[derive(Clone, Copy, Debug)]
pub struct CameraState {
    pub position: Vector3<f32>,
    pub direction: Vector3<f32>,
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct GizmoVertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
}

impl GizmoVertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<GizmoVertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: size_of::<[f32; 3]>() as u32,
            },
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum GizmoAxis {
    #[default]
    None,
    X,
    Y,
    Z,
    Center,
}

#[derive(Clone, Debug, Default)]
pub struct GizmoMesh {
    pub pipeline: RRPipeline,
    pub object_index: usize,
    pub vertices: Vec<GizmoVertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Option<vk::Buffer>,
    pub vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub index_buffer: Option<vk::Buffer>,
    pub index_buffer_memory: Option<vk::DeviceMemory>,
}

#[derive(Clone, Debug)]
pub struct GizmoPosition {
    pub position: Vector3<f32>,
}

impl Default for GizmoPosition {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GizmoSelectable {
    pub is_selected: bool,
    pub selected_axis: GizmoAxis,
}

#[derive(Clone, Debug)]
pub struct GizmoDraggable {
    pub drag_axis: GizmoAxis,
    pub just_selected: bool,
    pub initial_position: Vector3<f32>,
}

impl Default for GizmoDraggable {
    fn default() -> Self {
        Self {
            drag_axis: GizmoAxis::None,
            just_selected: false,
            initial_position: Vector3::new(0.0, 0.0, 0.0),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GizmoRayToModel {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Option<vk::Buffer>,
    pub vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub index_buffer: Option<vk::Buffer>,
    pub index_buffer_memory: Option<vk::DeviceMemory>,
}

#[derive(Clone, Debug, Default)]
pub struct GizmoVerticalLines {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Option<vk::Buffer>,
    pub vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub index_buffer: Option<vk::Buffer>,
    pub index_buffer_memory: Option<vk::DeviceMemory>,
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
