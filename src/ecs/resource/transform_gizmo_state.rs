#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TransformGizmoMode {
    #[default]
    Translate,
    Rotate,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CoordinateSpace {
    #[default]
    World,
    Local,
}

pub struct TransformGizmoState {
    pub mode: TransformGizmoMode,
    pub coordinate_space: CoordinateSpace,
    pub snap_enabled: bool,
    pub translate_snap_value: f32,
    pub rotate_snap_degrees: f32,
    pub scale_snap_value: f32,
    pub gizmo_scale: f32,
}

impl Default for TransformGizmoState {
    fn default() -> Self {
        Self {
            mode: TransformGizmoMode::Translate,
            coordinate_space: CoordinateSpace::World,
            snap_enabled: false,
            translate_snap_value: 0.5,
            rotate_snap_degrees: 15.0,
            scale_snap_value: 0.1,
            gizmo_scale: 0.08,
        }
    }
}
