/// Burst schedule, per-burst envelope and the seeded reshaping of the channel over time.
#[derive(Clone, Debug, PartialEq)]
pub struct LightningTiming {
    pub seed: u32,
    pub burst_start: f32,
    pub burst_interval: f32,
    pub burst_jitter: f32,
    pub burst_count: u32,
    pub attack_time: f32,
    pub sustain_time: f32,
    pub release_time: f32,
    pub stroke_count: u32,
    pub stroke_interval: f32,
    pub stroke_decay: f32,
    pub flicker_amplitude: f32,
    pub flicker_period: f32,
    pub reseed_level: u32,
    pub reseed_period: f32,
    pub charge_ramp: f32,
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
