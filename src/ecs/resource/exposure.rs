#[derive(Clone, Debug)]
pub struct Exposure {
    pub ev100: f32,
    pub exposure_value: f32,
}

impl Default for Exposure {
    fn default() -> Self {
        Self {
            ev100: -0.263,
            exposure_value: 1.0,
        }
    }
}
