#[derive(Clone, Debug)]
pub struct LensEffects {
    pub vignette_enabled: bool,
    pub vignette_intensity: f32,
    pub chromatic_aberration_enabled: bool,
    pub chromatic_aberration_intensity: f32,
}

impl Default for LensEffects {
    fn default() -> Self {
        Self {
            vignette_enabled: false,
            vignette_intensity: 0.3,
            chromatic_aberration_enabled: false,
            chromatic_aberration_intensity: 0.005,
        }
    }
}
