#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlySpeedModifier {
    #[default]
    Normal,
    Fast,
    Slow,
}

impl FlySpeedModifier {
    pub const FAST_FACTOR: f32 = 3.0;

    pub fn factor(self) -> f32 {
        match self {
            FlySpeedModifier::Normal => 1.0,
            FlySpeedModifier::Fast => Self::FAST_FACTOR,
            FlySpeedModifier::Slow => 1.0 / Self::FAST_FACTOR,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CameraFlyInput {
    pub forward: f32,
    pub right: f32,
    pub up: f32,
    pub speed_modifier: FlySpeedModifier,
    pub delta_seconds: f32,
}
