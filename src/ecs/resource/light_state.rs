use cgmath::Vector3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistanceAttenuation {
    Enabled,
    Disabled,
}

impl DistanceAttenuation {
    pub fn is_enabled(self) -> bool {
        self == Self::Enabled
    }

    pub fn as_int(self) -> i32 {
        if self == Self::Enabled {
            1
        } else {
            0
        }
    }
}

#[derive(Clone, Debug)]
pub struct LightState {
    pub light_position: Vector3<f32>,
    pub shadow_strength: f32,
    pub shadow_normal_offset: f32,
    pub distance_attenuation: DistanceAttenuation,
}

impl Default for LightState {
    fn default() -> Self {
        Self {
            light_position: Vector3::new(1.0, 1.0, 1.0),
            shadow_strength: 1.0,
            shadow_normal_offset: 0.5,
            distance_attenuation: DistanceAttenuation::Disabled,
        }
    }
}
