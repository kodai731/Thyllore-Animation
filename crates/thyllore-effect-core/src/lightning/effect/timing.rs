use crate::lightning::LightningParameterOwner;
/// Burst schedule, per-burst envelope and the seeded reshaping of the channel over time.
#[derive(Clone, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = LightningParameterOwner, owner = Frame, group = "timing")]
pub struct LightningTiming {
    /// Seed of the deterministic hash that shapes the channel and the burst jitter
    #[persist(ui(min = 0.0, max = 9999.0, format = "%.0f"))]
    pub seed: u32,
    #[persist(curve, ui(min = 0.0, max = 10.0))]
    pub burst_start: f32,
    #[persist(curve, ui(primary, min = 0.01, max = 10.0))]
    pub burst_interval: f32,
    /// Fraction of burst_interval the burst start is randomly shifted by
    #[persist(curve, ui(min = 0.0, max = 1.0))]
    pub burst_jitter: f32,
    /// Number of bursts to play; 0 repeats forever
    #[persist(curve, ui(min = 0.0, max = 16.0, format = "%.0f"))]
    pub burst_count: u32,
    #[persist(curve, ui(min = 0.0, max = 1.0, format = "%.3f"))]
    pub attack_time: f32,
    #[persist(curve, ui(min = 0.0, max = 1.0, format = "%.3f"))]
    pub sustain_time: f32,
    #[persist(curve, ui(min = 0.0, max = 2.0, format = "%.3f"))]
    pub release_time: f32,
    /// Return strokes per burst, played stroke_interval apart
    #[persist(curve, ui(min = 1.0, max = 8.0, format = "%.0f"))]
    pub stroke_count: u32,
    #[persist(curve, ui(min = 0.001, max = 1.0, format = "%.3f"))]
    pub stroke_interval: f32,
    /// Intensity of each return stroke relative to the previous one
    #[persist(curve, ui(min = 0.0, max = 1.0))]
    pub stroke_decay: f32,
    #[persist(curve, ui(min = 0.0, max = 1.0))]
    pub flicker_amplitude: f32,
    #[persist(curve, ui(min = 0.001, max = 1.0, format = "%.3f"))]
    pub flicker_period: f32,
    /// Subdivision level above which the channel is reshaped every reseed_period
    #[persist(curve, ui(min = 0.0, max = 8.0, format = "%.0f"))]
    pub reseed_level: u32,
    #[persist(curve, ui(min = 0.01, max = 10.0))]
    pub reseed_period: f32,
    /// Seconds of dim leader glow before the first return stroke; 0 strikes instantly
    #[persist(curve, ui(min = 0.0, max = 2.0))]
    pub charge_ramp: f32,
    /// Seconds the discharge takes to reach its end; 0 draws it whole at once
    #[persist(curve, ui(min = 0.0, max = 1.0, format = "%.3f"))]
    pub growth_time: f32,
}

impl Default for LightningTiming {
    fn default() -> Self {
        Self {
            seed: 0,
            burst_start: 0.0,
            burst_interval: 2.0,
            burst_jitter: 0.2,
            burst_count: 1,
            attack_time: 0.01,
            sustain_time: 0.04,
            release_time: 0.12,
            stroke_count: 1,
            stroke_interval: 0.05,
            stroke_decay: 0.6,
            flicker_amplitude: 0.25,
            flicker_period: 0.03,
            reseed_level: 2,
            reseed_period: 1.0,
            charge_ramp: 0.0,
            growth_time: 0.0,
        }
    }
}
