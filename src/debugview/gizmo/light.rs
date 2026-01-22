use crate::ecs::component::{
    GizmoDraggable, GizmoMesh, GizmoPosition, GizmoSelectable, LineMesh,
};

pub use crate::ecs::component::GizmoAxis as LightGizmoAxis;

#[derive(Clone, Debug, Default)]
pub struct LightGizmoData {
    pub mesh: GizmoMesh,
    pub position: GizmoPosition,
    pub selectable: GizmoSelectable,
    pub draggable: GizmoDraggable,
    pub ray_to_model: LineMesh,
    pub vertical_lines: LineMesh,
}

