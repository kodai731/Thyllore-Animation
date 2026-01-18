use crate::ecs::components::{GizmoMesh, GizmoVertex};
use crate::vulkanr::pipeline::RRPipeline;

pub use crate::ecs::components::GizmoVertex as GridGizmoVertex;

#[derive(Clone, Debug, Default)]
pub struct GridGizmoData {
    pub mesh: GizmoMesh,
}

impl GridGizmoData {
    pub fn new() -> Self {
        let axis_length = 0.15;

        let vertices = vec![
            GizmoVertex { pos: [0.0, 0.0, 0.0], color: [1.0, 1.0, 1.0] },
            GizmoVertex { pos: [axis_length, 0.0, 0.0], color: [1.0, 0.0, 0.0] },
            GizmoVertex { pos: [0.0, axis_length, 0.0], color: [0.0, 1.0, 0.0] },
            GizmoVertex { pos: [0.0, 0.0, axis_length], color: [0.0, 0.0, 1.0] },
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
        }
    }
}
