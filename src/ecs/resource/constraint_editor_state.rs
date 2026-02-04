pub struct ConstraintEditorState {
    pub add_type_index: i32,
}

impl Default for ConstraintEditorState {
    fn default() -> Self {
        Self { add_type_index: 3 }
    }
}
