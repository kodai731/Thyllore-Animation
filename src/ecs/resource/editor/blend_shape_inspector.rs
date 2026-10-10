use std::collections::HashMap;

#[derive(Default)]
pub struct BlendShapeInspectorState {
    pub search: String,
    pub mirror_edit: bool,
    pub expanded: HashMap<String, bool>,
}
