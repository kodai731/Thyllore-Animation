#[derive(Clone, Debug, PartialEq)]
pub struct LightningSurround {
    pub light_gain: f32,
    pub light_color: [f32; 3],
    pub light_height_fraction: f32,
    pub impact_radius: f32,
    pub impact_decay: f32,
    pub thunder_speed: f32,
}

impl Default for LightningSurround {
    fn default() -> Self {
        Self {
            light_gain: 0.0,
            light_color: [0.85, 0.92, 1.0],
            light_height_fraction: 0.5,
            impact_radius: 0.0,
            impact_decay: 0.5,
            thunder_speed: 343.0,
        }
    }
}
