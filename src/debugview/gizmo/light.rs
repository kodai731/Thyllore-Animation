use crate::ecs::components::{
    GizmoDraggable, GizmoMesh, GizmoPosition, GizmoRayToModel, GizmoSelectable,
    GizmoVerticalLines,
};

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

