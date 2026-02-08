#[derive(Clone, Debug)]
pub struct DepthOfField {
    pub enabled: bool,
    pub focus_distance: f32,
    pub max_blur_radius: f32,
}

impl Default for DepthOfField {
    fn default() -> Self {
        Self {
            enabled: false,
            focus_distance: 10.0,
            max_blur_radius: 8.0,
        }
    }
}
