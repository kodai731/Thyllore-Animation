#[derive(Clone, Debug)]
pub struct AutoExposure {
    pub enabled: bool,
    pub min_ev: f32,
    pub max_ev: f32,
    pub adaptation_speed_up: f32,
    pub adaptation_speed_down: f32,
    pub low_percent: f32,
    pub high_percent: f32,
    pub min_log_luminance: f32,
    pub log_luminance_range: f32,
    pub saved_manual_exposure: Option<f32>,
}

impl Default for AutoExposure {
    fn default() -> Self {
        Self {
            enabled: true,
            min_ev: -4.0,
            max_ev: 16.0,
            adaptation_speed_up: 3.0,
            adaptation_speed_down: 1.0,
            low_percent: 0.1,
            high_percent: 0.9,
            min_log_luminance: -10.0,
            log_luminance_range: 22.0,
            saved_manual_exposure: None,
        }
    }
}
