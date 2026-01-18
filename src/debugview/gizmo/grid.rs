use crate::ecs::components::GizmoMesh;

pub use crate::ecs::components::GizmoVertex as GridGizmoVertex;

#[derive(Clone, Debug, Default)]
pub struct GridGizmoData {
    pub mesh: GizmoMesh,
}
