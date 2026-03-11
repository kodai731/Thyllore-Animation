use cgmath::Vector3;

#[derive(Clone, Debug)]
pub struct LightState {
    pub light_position: Vector3<f32>,
    pub shadow_strength: f32,
    pub shadow_normal_offset: f32,
    pub enable_distance_attenuation: bool,
}

impl Default for LightState {
    fn default() -> Self {
        Self {
            light_position: Vector3::new(1.0, 1.0, 1.0),
            shadow_strength: 1.0,
            shadow_normal_offset: 0.5,
            enable_distance_attenuation: false,
        }
    }
}
