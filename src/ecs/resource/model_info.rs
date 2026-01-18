#[derive(Clone, Debug)]
pub struct ModelInfo {
    pub has_skinned_meshes: bool,
    pub node_animation_scale: f32,
}

impl ModelInfo {
    pub fn new() -> Self {
        Self {
            has_skinned_meshes: false,
            node_animation_scale: 1.0,
        }
    }
}

impl Default for ModelInfo {
    fn default() -> Self {
        Self::new()
    }
}
