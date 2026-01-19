use std::mem::size_of;

use cgmath::Vector3;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::PipelineId;
use crate::render::{IndexBufferHandle, VertexBufferHandle};
use crate::vulkanr::data::Vertex;

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
    pub pipeline_id: Option<PipelineId>,
    pub object_index: usize,
    pub vertices: Vec<GizmoVertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer_handle: VertexBufferHandle,
    pub index_buffer_handle: IndexBufferHandle,
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
    pub vertex_buffer_handle: VertexBufferHandle,
    pub index_buffer_handle: IndexBufferHandle,
}

#[derive(Clone, Debug, Default)]
pub struct GizmoVerticalLines {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer_handle: VertexBufferHandle,
    pub index_buffer_handle: IndexBufferHandle,
}
