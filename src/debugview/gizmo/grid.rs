use crate::ecs::component::GizmoMesh;

pub use crate::ecs::component::GizmoVertex as GridGizmoVertex;

#[derive(Clone, Debug, Default)]
pub struct GridGizmoData {
    pub mesh: GizmoMesh,
}
