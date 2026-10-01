/// Named presets of an effect, applied onto an existing effect value.
pub trait EffectPresets {
    const PRESET_NAMES: &'static [&'static str];
    fn apply_preset(&mut self, name: &str) -> bool;
}

/// The pose every pack call sets: local time plus the world placement.
pub trait Placement {
    fn set_placement(&mut self, time: f32, position: [f32; 3], rotation: [f32; 4]);
}
