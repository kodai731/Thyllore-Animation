use cgmath::{Matrix4, Quaternion, Vector3};

use crate::wind::ownership::WindParameterOwner;

/// Animation-curve codes this effect owns; every `#[persist(code = N)]` below lies inside it.
pub const WIND_SCALAR_CODES: thyllore_scene_core::ScalarCodeBlock =
    thyllore_scene_core::ScalarCodeBlock::new(512, 256);

#[derive(Clone, Debug, PartialEq, crate::UboPack, thyllore_scene_core::SceneFields)]
#[ubo(target = crate::WindUBO)]
#[scene(key = "wind_tornado", tag = WindParameterOwner, owner = Frame, tags = WIND_PARAMETER_OWNERSHIP, snapshot = wind_parameter_snapshot, scalars = WIND_SCALAR_PARAMS, ui = WIND_UI_PARAMS, overwrite = overwrite_wind_persisted_fields)]
pub struct WindTornadoEffect {
    #[ubo("optics.z")]
    #[runtime(ui(min = 0.0, max = 100.0))]
    pub time: f32,
    #[runtime(ui(primary, min = 0.0, max = 4.0))]
    pub time_scale: f32,
    #[runtime(ui(min = -100.0, max = 100.0))]
    pub time_offset: f32,
    #[persist(as = [f32; 3])]
    pub position: Vector3<f32>,
    #[persist(as = [f32; 4], with = crate::scene_convert::quaternion_wxyz)]
    pub rotation: Quaternion<f32>,
    #[persist(code = 512, debug_range = (0.5, 10.0), ui(primary, min = 0.1, max = 20.0, group = "shape"))]
    pub column_height: f32,
    #[persist(code = 513, debug_range = (0.05, 5.0), ui(primary, min = 0.01, max = 10.0, format = "%.3f", group = "shape"))]
    pub wall_radius_base: f32,
    #[persist(code = 514, debug_range = (0.05, 5.0), ui(min = 0.01, max = 10.0, format = "%.3f", group = "shape"))]
    pub wall_radius_top: f32,
    /// Half width of the wall shell in squared-radius units; the radial thickness is about wall_width_q / (2 R)
    #[persist(code = 515, debug_range = (0.01, 2.0), ui(min = 0.001, max = 5.0, format = "%.3f", group = "shape"))]
    pub wall_width_q: f32,
    /// Fraction of the height over which the density fades to zero at the top
    #[persist(code = 517, debug_range = (0.05, 1.0), ui(min = 0.01, max = 1.0, group = "shape"))]
    pub top_fade: f32,
    /// Extinction coefficient per meter at unit shell density
    #[persist(code = 518, debug_range = (0.0, 20.0), ui(primary, min = 0.0, max = 50.0, group = "density"))]
    pub density: f32,
    #[persist(code = 516, ui(min = 0.0, max = 4.0, group = "density"))]
    pub wall_strength: f32,
    /// Height fraction of the column at t = 0; the top rises to 1 over rise_duration
    #[persist(code = 525, ui(min = 0.0, max = 1.0, group = "motion"))]
    pub rise_initial_height: f32,
    /// Seconds of the smoothstep rise from rise_initial_height to the full height
    #[persist(code = 526, ui(primary, min = 0.1, max = 10.0, group = "motion"))]
    pub rise_duration: f32,
    /// Time in seconds when wall spreading begins
    #[persist(code = 527, ui(min = 0.0, max = 10.0, group = "motion"))]
    #[ubo("streak2.w")]
    pub spread_start: f32,
    /// Outward drift of the wall in squared-radius units: 2 * spread_rate * (t - spread_start)
    #[persist(code = 528, ui(min = 0.0, max = 5.0, group = "motion"))]
    #[ubo("lighting.w")]
    pub spread_rate: f32,
    /// Time in seconds when wall dissipation begins
    #[persist(code = 529, ui(min = 0.0, max = 10.0, group = "motion"))]
    pub dissipate_start: f32,
    /// Time constant of the wall strength decay; 0 keeps the wall at full strength
    #[persist(code = 530, ui(min = 0.0, max = 10.0, group = "motion"))]
    pub dissipate_time: f32,
    /// Circulation of the Rankine vortex; used for streak phase computation
    #[persist(code = 531, ui(primary, min = 0.0, max = 100.0, group = "motion"))]
    #[ubo("lighting.z")]
    pub circulation: f32,
    /// Number of spiral streaks (m in the phase)
    #[persist(
        code = 532,
        ui(min = 1.0, max = 16.0, format = "%.1f", group = "motion")
    )]
    pub streak_order: f32,
    /// Twist of the spiral streaks (kappa in the phase)
    #[persist(
        code = 533,
        ui(min = 0.0, max = 20.0, format = "%.1f", group = "motion")
    )]
    pub streak_twist: f32,
    /// Rise speed of the spiral streaks (omega_z in the phase)
    #[persist(
        code = 534,
        ui(min = 0.0, max = 10.0, format = "%.1f", group = "motion")
    )]
    pub streak_rise_speed: f32,
    /// Amplitude of the streak modulation; 0 disables streaks (identity)
    #[persist(code = 535, ui(min = 0.0, max = 1.0, group = "motion"))]
    pub streak_amplitude: f32,
    /// Amplitude of the volumetric eddy; 0 disables eddies (identity)
    #[persist(code = 536, ui(primary, min = 0.0, max = 1.0, group = "eddy"))]
    pub eddy_amplitude: f32,
    /// Eddy cell size in theta (angular) direction
    #[persist(code = 537, ui(min = 0.01, max = 2.0, group = "eddy"))]
    pub eddy_cell_theta: f32,
    /// Eddy cell size in height (vertical) direction
    #[persist(code = 538, ui(min = 0.01, max = 2.0, group = "eddy"))]
    pub eddy_cell_height: f32,
    /// Eddy cell size in radial direction
    #[persist(code = 539, ui(min = 0.01, max = 1.0, group = "eddy"))]
    pub eddy_cell_radial: f32,
    /// Shear of the eddy field; 0 is no shear (pure translation)
    #[persist(code = 540, ui(min = 0.0, max = 1.0, group = "eddy"))]
    pub eddy_shear: f32,
    /// Width of rotation speed applied to octave and reseed layers. 0 means all octaves have the same speed.
    #[persist(code = 541, ui(min = 0.0, max = 1.0, group = "eddy"))]
    #[ubo("streak2.z")]
    pub eddy_speed_spread: f32,
    /// Rise speed of the eddy field (omega_z in the phase)
    #[persist(code = 542, ui(min = 0.0, max = 10.0, format = "%.1f", group = "eddy"))]
    pub eddy_rise_speed: f32,
    /// Period of the eddy reseed (time between resamples)
    #[persist(code = 543, ui(min = 0.1, max = 10.0, format = "%.1f", group = "eddy"))]
    pub eddy_reseed_period: f32,
    /// Noise floor carved out of the eddy field; density reaches 0 where noise falls below it, 0 keeps the smooth modulation (identity)
    #[persist(code = 544, ui(min = 0.0, max = 0.95, group = "eddy"))]
    pub eddy_erosion: f32,
    /// Number of puff clumps around the theta (angular) direction; 0 means no puffs (identity)
    #[persist(code = 545, ui(min = 0.0, max = 16.0, format = "%.0f", group = "eddy"))]
    pub puff_count_theta: u32,
    /// Number of puff clumps along the height (vertical) direction; 0 means no puffs (identity)
    #[persist(code = 546, ui(min = 0.0, max = 16.0, format = "%.0f", group = "eddy"))]
    pub puff_count_height: u32,
    /// Radius of each puff clump in world units
    #[persist(code = 547, ui(min = 0.01, max = 1.0, group = "eddy"))]
    pub puff_radius: f32,
    /// Fractional jitter of puff radius for organic variation
    #[persist(code = 548, ui(min = 0.0, max = 1.0, group = "eddy"))]
    pub puff_radius_jitter: f32,
    /// Radial offset of puff clumps from the wall in q space
    #[persist(code = 549, ui(min = 0.0, max = 1.0, group = "eddy"))]
    pub puff_offset_q: f32,
    /// Strength of the puff density contribution relative to the wall
    #[persist(code = 550, ui(min = 0.0, max = 4.0, group = "eddy"))]
    #[ubo("puff_params.y")]
    pub puff_strength: f32,
    /// Rise speed of puff clumps along the tornado height
    #[persist(code = 551, ui(min = 0.0, max = 10.0, format = "%.1f", group = "eddy"))]
    pub puff_rise_speed: f32,
    /// Single-scattering albedo of the dust
    #[persist(code = [519, 520, 521], ui(primary, min = 0.0, max = 1.0, group = "look"))]
    #[ubo("albedo.xyz")]
    pub albedo: [f32; 3],
    #[persist(code = 522, ui(min = 0.0, max = 5.0, group = "look"))]
    #[ubo("optics.y")]
    pub ambient_brightness: f32,
    /// Henyey-Greenstein anisotropy of the dust; positive scatters forward
    #[persist(code = 523, ui(min = -0.95, max = 0.95, group = "look"))]
    #[ubo("lighting.x")]
    pub phase_g: f32,
    /// Radiance of the sun used by the single-scattering source term
    #[persist(code = 524, ui(min = 0.0, max = 10.0, group = "look"))]
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

impl crate::EffectPresets for WindTornadoEffect {
    const PRESET_NAMES: &'static [&'static str] = crate::WIND_PRESET_NAMES;

    fn apply_preset(&mut self, name: &str) -> bool {
        crate::apply_wind_preset(self, name)
    }
}

impl crate::Placement for WindTornadoEffect {
    fn set_placement(&mut self, time: f32, position: [f32; 3], rotation: [f32; 4]) {
        self.time = time;
        self.position = Vector3::from(position);
        crate::scene_convert::quaternion_wxyz::set(&mut self.rotation, rotation);
    }
}
