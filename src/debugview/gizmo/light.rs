use cgmath::Vector3;

use crate::ecs::components::{
    GizmoDraggable, GizmoMesh, GizmoPosition, GizmoRayToModel, GizmoSelectable, GizmoVertex,
    GizmoVerticalLines,
};
use crate::vulkanr::pipeline::RRPipeline;

pub use crate::ecs::components::GizmoAxis as LightGizmoAxis;

#[derive(Clone, Debug, Default)]
pub struct LightGizmoData {
    pub mesh: GizmoMesh,
    pub position: GizmoPosition,
    pub selectable: GizmoSelectable,
    pub draggable: GizmoDraggable,
    pub ray_to_model: GizmoRayToModel,
    pub vertical_lines: GizmoVerticalLines,
}

impl LightGizmoData {
    pub fn new(position: Vector3<f32>) -> Self {
        let axis_length = 1.0;
        let yellow = [1.0, 1.0, 0.0];

        let vertices = vec![
            GizmoVertex {
                pos: [0.0, 0.0, 0.0],
                color: yellow,
            },
            GizmoVertex {
                pos: [axis_length, 0.0, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            GizmoVertex {
                pos: [0.0, axis_length, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            GizmoVertex {
                pos: [0.0, 0.0, axis_length],
                color: [0.0, 0.0, 1.0],
            },
        ];

        let indices = vec![0, 1, 0, 2, 0, 3];

        Self {
            mesh: GizmoMesh {
                pipeline: RRPipeline::default(),
                object_index: 0,
                vertices,
                indices,
                vertex_buffer: None,
                vertex_buffer_memory: None,
                index_buffer: None,
                index_buffer_memory: None,
            },
            position: GizmoPosition { position },
            selectable: GizmoSelectable::default(),
            draggable: GizmoDraggable::default(),
            ray_to_model: GizmoRayToModel::default(),
            vertical_lines: GizmoVerticalLines::default(),
        }
    }
}
