use glam::{Mat4, Vec2};

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

pub struct ProjectionData {
    pub view: Mat4,
    pub proj: Mat4,
    pub screen_size: Vec2,
    pub aspect: f32,
}
