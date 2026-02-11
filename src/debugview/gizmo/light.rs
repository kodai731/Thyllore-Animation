use crate::ecs::component::{GizmoDraggable, GizmoPosition, GizmoSelectable, LineMesh, RenderInfo};

#[derive(Clone, Debug, Default)]
pub struct LightGizmoData {
    pub mesh: LineMesh,
    pub render_info: RenderInfo,
    pub position: GizmoPosition,
    pub selectable: GizmoSelectable,
    pub draggable: GizmoDraggable,
    pub ray_to_model: LineMesh,
    pub vertical_lines: LineMesh,
}
