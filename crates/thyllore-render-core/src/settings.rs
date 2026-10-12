#[derive(Clone, Debug)]
pub struct BloomSettings {
    pub enabled: bool,
    pub intensity: f32,
    pub threshold: f32,
    pub knee: f32,
    pub mip_count: u32,
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.04,
            threshold: 1.0,
            knee: 0.5,
            mip_count: 5,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DepthOfField {
    pub enabled: bool,
    pub focus_distance: f32,
    pub max_blur_radius: f32,
}

impl Default for DepthOfField {
    fn default() -> Self {
        Self {
            enabled: false,
            focus_distance: 10.0,
            max_blur_radius: 8.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(i32)]
pub enum ToneMapOperator {
    None = 0,
    AcesFilmic = 1,
    Reinhard = 2,
}

#[derive(Clone, Debug)]
pub struct ToneMapping {
    pub enabled: bool,
    pub operator: ToneMapOperator,
    pub gamma: f32,
}

impl Default for ToneMapping {
    fn default() -> Self {
        Self {
            enabled: true,
            operator: ToneMapOperator::AcesFilmic,
            gamma: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Exposure {
    pub ev100: f32,
    pub exposure_value: f32,
    /// Scene luminance the auto exposure metered this frame; 0 while auto exposure is off.
    pub metered_luminance: f32,
}

impl Exposure {
    /// Unity HDRP `ConvertEV100ToExposure`: exposure = 1 / (1.2 * 2^EV100).
    pub fn ev100_from_exposure_value(exposure_value: f32) -> f32 {
        -(1.2 * exposure_value.max(f32::MIN_POSITIVE)).log2()
    }
}

impl Default for Exposure {
    fn default() -> Self {
        Self {
            ev100: -0.263,
            exposure_value: 1.0,
            metered_luminance: 0.0,
        }
    }
}

/// Brightness of the whole Scene View in EV, applied on top of the exposure; positive brightens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneBrightness {
    pub ev: f32,
}

impl Default for SceneBrightness {
    fn default() -> Self {
        Self { ev: 0.0 }
    }
}

impl Exposure {
    /// The multiplier the tone map applies: the exposure shifted by the Scene View brightness.
    pub fn display_multiplier(&self, brightness: SceneBrightness) -> f32 {
        self.exposure_value * brightness.ev.exp2()
    }
}

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

/// Where the exposure comes from: the scene light, or a histogram of the HDR image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExposureMetering {
    /// No image metering: the exposure is derived from the scene light, so it never reacts to
    /// what is in the frame.
    Light,
    /// Eye adaptation from the HDR histogram.
    Scene,
}

impl ExposureMetering {
    pub const ALL: [ExposureMetering; 2] = [ExposureMetering::Light, ExposureMetering::Scene];

    /// Stable name persisted in scene files.
    pub fn name(self) -> &'static str {
        match self {
            ExposureMetering::Light => "Light",
            ExposureMetering::Scene => "Scene",
        }
    }

    pub fn uses_histogram(self) -> bool {
        match self {
            ExposureMetering::Light => false,
            ExposureMetering::Scene => true,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|metering| metering.name() == name)
    }
}

#[derive(Clone, Debug)]
pub struct AutoExposure {
    pub enabled: bool,
    pub metering: ExposureMetering,
    /// Display value (linear) of a white diffuse surface facing the light under `Light` metering.
    pub white_level: f32,
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

/// Unreal's metered key, 18 % calibration times its 1 EV default bias, is the white level; the
/// histogram limits -1 / 14 EV, 40-90 % and speeds 3 / 1 are Unity HDRP's `Exposure` defaults.
impl Default for AutoExposure {
    fn default() -> Self {
        Self {
            enabled: true,
            metering: ExposureMetering::Light,
            white_level: 0.36,
            min_ev: -1.0,
            max_ev: 14.0,
            adaptation_speed_up: 3.0,
            adaptation_speed_down: 1.0,
            low_percent: 0.4,
            high_percent: 0.9,
            min_log_luminance: -10.0,
            log_luminance_range: 22.0,
            saved_manual_exposure: None,
        }
    }
}

impl AutoExposure {
    pub fn uses_histogram(&self) -> bool {
        self.enabled && self.metering.uses_histogram()
    }

    /// Exposure that puts a white lit surface at `white_level`.
    pub fn light_referenced_exposure(&self, lit_white_radiance: f32) -> Exposure {
        let exposure_value = self.white_level / lit_white_radiance.max(f32::MIN_POSITIVE);
        Exposure {
            ev100: Exposure::ev100_from_exposure_value(exposure_value),
            exposure_value,
            metered_luminance: lit_white_radiance,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PhysicalCameraParameters {
    pub focal_length_mm: f32,
    pub sensor_height_mm: f32,
    pub aperture_f_stops: f32,
    pub shutter_speed_s: f32,
    pub sensitivity_iso: f32,
}

impl Default for PhysicalCameraParameters {
    fn default() -> Self {
        Self {
            focal_length_mm: 35.0,
            sensor_height_mm: 18.66,
            aperture_f_stops: 16.0,
            shutter_speed_s: 1.0 / 125.0,
            sensitivity_iso: 100.0,
        }
    }
}

impl ToneMapOperator {
    pub const ALL: [ToneMapOperator; 3] = [
        ToneMapOperator::None,
        ToneMapOperator::AcesFilmic,
        ToneMapOperator::Reinhard,
    ];

    /// Stable name persisted in scene files.
    pub fn name(self) -> &'static str {
        match self {
            ToneMapOperator::None => "None",
            ToneMapOperator::AcesFilmic => "AcesFilmic",
            ToneMapOperator::Reinhard => "Reinhard",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|operator| operator.name() == name)
    }
}
