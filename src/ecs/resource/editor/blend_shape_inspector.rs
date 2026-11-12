#[derive(Default)]
pub struct BlendShapeInspectorState {
    pub search: String,
    pub mirror_edit: bool,
}

crate::startup_resource!(BlendShapeInspectorState, Editor);
