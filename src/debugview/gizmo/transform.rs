use cgmath::{Quaternion, Vector3};

use crate::animation::BoneId;
use crate::ecs::component::{GizmoDraggable, GizmoPosition, GizmoSelectable, LineMesh, RenderInfo};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TransformGizmoHandle {
    #[default]
    None,
    AxisX,
    AxisY,
    AxisZ,
    PlaneXY,
    PlaneXZ,
    PlaneYZ,
    Center,
    RingX,
    RingY,
    RingZ,
    Trackball,
}

#[derive(Clone, Debug)]
pub struct TransformGizmoData {
    pub visible: bool,
    pub position: GizmoPosition,
    pub selectable: GizmoSelectable,
    pub draggable: GizmoDraggable,
    pub active_handle: TransformGizmoHandle,
    pub drag_active: bool,
    // Line-based meshes (axis shafts, rings)
    pub line_mesh: LineMesh,
    // Solid triangle meshes (cone tips, cubes, plane quads)
    pub solid_mesh: LineMesh,
    // Render info for line pipeline (no-depth line)
    pub line_render_info: RenderInfo,
    // Render info for solid pipeline (no-depth triangle)
    pub solid_render_info: RenderInfo,
    // Drag state
    pub drag_start_position: Vector3<f32>,
    pub drag_start_rotation: Quaternion<f32>,
    pub drag_start_scale: Vector3<f32>,
    pub drag_plane_normal: Vector3<f32>,
    pub drag_initial_hit: Vector3<f32>,
    pub drag_initial_angle: f32,
    // Target tracking
    pub target_bone_id: Option<BoneId>,
}

impl Default for TransformGizmoData {
    fn default() -> Self {
        Self {
            visible: false,
            position: GizmoPosition::default(),
            selectable: GizmoSelectable::default(),
            draggable: GizmoDraggable::default(),
            active_handle: TransformGizmoHandle::None,
            drag_active: false,
            line_mesh: LineMesh::default(),
            solid_mesh: LineMesh::default(),
            line_render_info: RenderInfo::default(),
            solid_render_info: RenderInfo::default(),
            drag_start_position: Vector3::new(0.0, 0.0, 0.0),
            drag_start_rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            drag_start_scale: Vector3::new(1.0, 1.0, 1.0),
            drag_plane_normal: Vector3::new(0.0, 1.0, 0.0),
            drag_initial_hit: Vector3::new(0.0, 0.0, 0.0),
            drag_initial_angle: 0.0,
            target_bone_id: None,
        }
    }
}
