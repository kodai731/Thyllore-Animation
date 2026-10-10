#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionPreference {
    #[default]
    Full,
    Reduced,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UiSettings {
    pub reduced_motion: MotionPreference,
}
