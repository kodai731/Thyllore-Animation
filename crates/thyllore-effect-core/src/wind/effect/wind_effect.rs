use cgmath::{Matrix4, Quaternion, Vector3};

use super::scene_format::*;
use crate::wind::ownership::WindParameterOwner;

#[derive(
    Clone,
    Debug,
    PartialEq,
    thyllore_effect_derive::UboPack,
    thyllore_effect_derive::SceneFormat,
    thyllore_effect_derive::PyEffect,
)]
#[ubo(target = crate::WindUBO)]
#[py_effect(presets = crate::WIND_PRESET_NAMES, apply_preset = crate::apply_wind_preset)]
#[scene(record = WindSceneRecord, tag = WindParameterOwner, key = "wind_tornado", tags = WIND_PARAMETER_OWNERSHIP, snapshot = wind_parameter_snapshot, scalars = WIND_SCALAR_PARAMS, ui = WIND_UI_PARAMS, overwrite = overwrite_wind_persisted_fields)]
pub struct WindTornadoEffect {
    #[ubo("optics.z")]
    #[runtime(ui(min = 0.0, max = 100.0, format = "%.2f"))]
    pub time: f32,
    #[runtime(ui(primary, min = 0.0, max = 4.0, format = "%.2f"))]
    pub time_scale: f32,
    #[runtime(ui(min = -100.0, max = 100.0, format = "%.2f"))]
    pub time_offset: f32,
    #[persist(owner = Frame, as = [f32; 3], get = wind_position_get, set = wind_position_set)]
    pub position: Vector3<f32>,
    #[persist(owner = Frame, as = [f32; 4], get = wind_rotation_get, set = wind_rotation_set)]
    pub rotation: Quaternion<f32>,
    #[persist(owner = Frame, ui(primary, min = 0.1, max = 20.0, format = "%.2f", group = "shape"))]
    pub column_height: f32,
    #[persist(owner = Frame, ui(primary, min = 0.01, max = 10.0, format = "%.3f", group = "shape"))]
    pub wall_radius_base: f32,
    #[persist(owner = Frame, ui(min = 0.01, max = 10.0, format = "%.3f", group = "shape"))]
    pub wall_radius_top: f32,
    #[persist(owner = Frame, ui(min = 0.001, max = 5.0, format = "%.3f", tooltip = "Half width of the wall shell in squared-radius units; the radial thickness is about wall_width_q / (2 R)", group = "shape"))]
    pub wall_width_q: f32,
    #[persist(owner = Frame, ui(min = 0.01, max = 1.0, format = "%.2f", tooltip = "Fraction of the height over which the density fades to zero at the top", group = "shape"))]
    pub top_fade: f32,
    #[persist(owner = Frame, ui(primary, min = 0.0, max = 50.0, format = "%.2f", tooltip = "Extinction coefficient per meter at unit shell density", group = "density"))]
    pub density: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 4.0, format = "%.2f", group = "density"))]
    pub wall_strength: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Height fraction of the column at t = 0; the top rises to 1 over rise_duration", group = "motion"))]
    pub rise_initial_height: f32,
    #[persist(owner = Frame, ui(primary, min = 0.1, max = 10.0, format = "%.2f", tooltip = "Seconds of the smoothstep rise from rise_initial_height to the full height", group = "motion"))]
    pub rise_duration: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 10.0, format = "%.2f", tooltip = "Time in seconds when wall spreading begins", group = "motion"))]
    #[ubo("streak2.w")]
    pub spread_start: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 5.0, format = "%.2f", tooltip = "Outward drift of the wall in squared-radius units: 2 * spread_rate * (t - spread_start)", group = "motion"))]
    #[ubo("lighting.w")]
    pub spread_rate: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 10.0, format = "%.2f", tooltip = "Time in seconds when wall dissipation begins", group = "motion"))]
    pub dissipate_start: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 10.0, format = "%.2f", tooltip = "Time constant of the wall strength decay; 0 keeps the wall at full strength", group = "motion"))]
    pub dissipate_time: f32,
    #[persist(owner = Frame, ui(primary, min = 0.0, max = 100.0, format = "%.2f", tooltip = "Circulation of the Rankine vortex; used for streak phase computation", group = "motion"))]
    #[ubo("lighting.z")]
    pub circulation: f32,
    #[persist(owner = Frame, ui(min = 1.0, max = 16.0, format = "%.1f", tooltip = "Number of spiral streaks (m in the phase)", group = "motion"))]
    pub streak_order: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 20.0, format = "%.1f", tooltip = "Twist of the spiral streaks (kappa in the phase)", group = "motion"))]
    pub streak_twist: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 10.0, format = "%.1f", tooltip = "Rise speed of the spiral streaks (omega_z in the phase)", group = "motion"))]
    pub streak_rise_speed: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Amplitude of the streak modulation; 0 disables streaks (identity)", group = "motion"))]
    pub streak_amplitude: f32,
    #[persist(owner = Frame, ui(primary, min = 0.0, max = 1.0, format = "%.2f", tooltip = "Amplitude of the volumetric eddy; 0 disables eddies (identity)", group = "eddy"))]
    pub eddy_amplitude: f32,
    #[persist(owner = Frame, ui(min = 0.01, max = 2.0, format = "%.2f", tooltip = "Eddy cell size in theta (angular) direction", group = "eddy"))]
    pub eddy_cell_theta: f32,
    #[persist(owner = Frame, ui(min = 0.01, max = 2.0, format = "%.2f", tooltip = "Eddy cell size in height (vertical) direction", group = "eddy"))]
    pub eddy_cell_height: f32,
    #[persist(owner = Frame, ui(min = 0.01, max = 1.0, format = "%.2f", tooltip = "Eddy cell size in radial direction", group = "eddy"))]
    pub eddy_cell_radial: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Shear of the eddy field; 0 is no shear (pure translation)", group = "eddy"))]
    pub eddy_shear: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Width of rotation speed applied to octave and reseed layers. 0 means all octaves have the same speed.", group = "eddy"))]
    #[ubo("streak2.z")]
    pub eddy_speed_spread: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 10.0, format = "%.1f", tooltip = "Rise speed of the eddy field (omega_z in the phase)", group = "eddy"))]
    pub eddy_rise_speed: f32,
    #[persist(owner = Frame, ui(min = 0.1, max = 10.0, format = "%.1f", tooltip = "Period of the eddy reseed (time between resamples)", group = "eddy"))]
    pub eddy_reseed_period: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 0.95, format = "%.2f", tooltip = "Noise floor carved out of the eddy field; density reaches 0 where noise falls below it, 0 keeps the smooth modulation (identity)", group = "eddy"))]
    pub eddy_erosion: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 16.0, format = "%.0f", tooltip = "Number of puff clumps around the theta (angular) direction; 0 means no puffs (identity)", group = "eddy"))]
    pub puff_count_theta: u32,
    #[persist(owner = Frame, ui(min = 0.0, max = 16.0, format = "%.0f", tooltip = "Number of puff clumps along the height (vertical) direction; 0 means no puffs (identity)", group = "eddy"))]
    pub puff_count_height: u32,
    #[persist(owner = Frame, ui(min = 0.01, max = 1.0, format = "%.2f", tooltip = "Radius of each puff clump in world units", group = "eddy"))]
    pub puff_radius: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Fractional jitter of puff radius for organic variation", group = "eddy"))]
    pub puff_radius_jitter: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Radial offset of puff clumps from the wall in q space", group = "eddy"))]
    pub puff_offset_q: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 4.0, format = "%.2f", tooltip = "Strength of the puff density contribution relative to the wall", group = "eddy"))]
    #[ubo("puff_params.y")]
    pub puff_strength: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 10.0, format = "%.1f", tooltip = "Rise speed of puff clumps along the tornado height", group = "eddy"))]
    pub puff_rise_speed: f32,
    #[persist(owner = Frame, scalars = rgb, ui(primary, kind = Color, min = 0.0, max = 1.0, format = "%.2f", tooltip = "Single-scattering albedo of the dust", group = "look"))]
    #[ubo("albedo.xyz")]
    pub albedo: [f32; 3],
    #[persist(owner = Frame, ui(min = 0.0, max = 5.0, format = "%.2f", group = "look"))]
    #[ubo("optics.y")]
    pub ambient_brightness: f32,
    #[persist(owner = Frame, ui(min = -0.95, max = 0.95, format = "%.2f", tooltip = "Henyey-Greenstein anisotropy of the dust; positive scatters forward", group = "look"))]
    #[ubo("lighting.x")]
    pub phase_g: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 10.0, format = "%.2f", tooltip = "Radiance of the sun used by the single-scattering source term", group = "look"))]
    #[ubo("lighting.y")]
    pub sun_intensity: f32,
}

impl Default for WindTornadoEffect {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            time: 0.0,
            time_scale: 1.0,
            time_offset: 0.0,
            column_height: 2.0,
            wall_radius_base: 0.35,
            wall_radius_top: 0.6,
            wall_width_q: 0.08,
            wall_strength: 1.0,
            top_fade: 0.3,
            density: 4.0,
            albedo: [0.9, 0.93, 1.0],
            ambient_brightness: 1.0,
            phase_g: 0.6,
            sun_intensity: 1.0,
            rise_initial_height: 1.0,
            rise_duration: 1.0,
            spread_start: 0.0,
            spread_rate: 0.0,
            dissipate_start: 0.0,
            dissipate_time: 0.0,
            circulation: 0.0,
            streak_order: 3.0,
            streak_twist: 4.0,
            streak_rise_speed: 1.0,
            streak_amplitude: 0.0,
            eddy_amplitude: 0.0,
            eddy_cell_theta: 0.4,
            eddy_cell_height: 0.3,
            eddy_cell_radial: 0.1,
            eddy_shear: 0.0,
            eddy_speed_spread: 0.0,
            eddy_rise_speed: 0.0,
            eddy_reseed_period: 1.0,
            eddy_erosion: 0.0,
            puff_count_theta: 0,
            puff_count_height: 0,
            puff_radius: 0.15,
            puff_radius_jitter: 0.4,
            puff_offset_q: 0.5,
            puff_strength: 1.0,
            puff_rise_speed: 0.3,
        }
    }
}

pub fn build_wind_model_matrix(effect: &WindTornadoEffect) -> Matrix4<f32> {
    Matrix4::from_translation(effect.position) * Matrix4::from(effect.rotation)
}
