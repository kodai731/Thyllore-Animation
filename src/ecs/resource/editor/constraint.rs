pub struct ConstraintEditorState {
    pub add_type_index: i32,
    pub bake_fps: f32,
}

impl Default for ConstraintEditorState {
    fn default() -> Self {
        Self {
            add_type_index: 3,
            bake_fps: 30.0,
        }
    }
}
