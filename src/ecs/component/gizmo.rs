use cgmath::Vector3;

use crate::ecs::resource::PipelineId;
use crate::render::{IndexBufferHandle, VertexBufferHandle};

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct GizmoVertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
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

