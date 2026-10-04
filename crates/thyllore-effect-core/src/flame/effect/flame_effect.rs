use crate::flame::*;
use cgmath::{Matrix4, Quaternion, Vector3};

/// Animation-curve codes this effect owns; every `#[persist(code = N)]` below lies inside it.
pub const FLAME_SCALAR_CODES: thyllore_scene_core::ScalarCodeBlock =
    thyllore_scene_core::ScalarCodeBlock::new(0, 256);

#[derive(Clone, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[scene(key = "flame", tag = ParameterOwner, owner = Frame, tags = PARAMETER_OWNERSHIP, snapshot = flame_parameter_snapshot, scalars = FLAME_SCALAR_PARAMS, ui = FLAME_UI_PARAMS, overwrite = overwrite_persisted_fields)]
pub struct FlameEffect {
    #[persist(as = [f32; 3])]
    pub position: Vector3<f32>,
    #[persist(as = [f32; 4], with = crate::scene_convert::quaternion_wxyz)]
    pub rotation: Quaternion<f32>,
    #[persist(code = 0, debug_range = (0.5, 4.0), ui(primary, min = 0.05, max = 10.0, group = "body"))]
    pub height: f32,
    #[persist(code = 1, debug_range = (0.2, 2.0), ui(primary, min = 0.05, max = 10.0, group = "body"))]
    pub radius: f32,
    #[persist(code = 3, owner = Style, debug_range = (0.5, 5.0))]
    pub sigma_t: f32,
    #[persist(code = 2, owner = Style, debug_range = (0.5, 5.0), ui(primary, min = 0.0, max = 10.0, group = "body"))]
    pub intensity: f32,
    #[nested]
    pub color: FlameColor,
    #[nested]
    pub noise: FlameNoise,
    #[runtime]
    pub time: f32,
    #[persist(ui(primary, min = 0.0, max = 4.0, group = "footer"))]
    pub time_scale: f32,
    #[persist]
    pub time_offset: f32,
    #[nested]
    pub warp: FlameWarp,
    #[nested]
    pub edge: FlameEdge,
    #[nested]
    pub wind: FlameWind,
    #[persist(owner = Style)]
    pub self_shadow_strength: f32,
    #[nested]
    pub envelope: FlameEnvelope,
    #[nested(runtime)]
    pub emitter: FlameEmitter,
    #[nested(runtime)]
    pub boundary: FlameBoundary,
    #[persist(owner = Shape)]
    pub radial_sharpness: f32,
    #[nested]
    pub contour: FlameContour,
    #[nested]
    pub carve: FlameCarve,
    #[nested]
    pub swirl: FlameSwirl,
    /// Medium spread toward the tip: noise features enlarge, drift outward and dissolve as they rise (0 = rigid scroll)
    #[persist(owner = Style, ui(min = 0.0, max = 3.0, group = "motion"))]
    pub spread_gain: f32,
    /// Flame density support radius: multiplier for the biweight support radius (how much extra space is allowed for carving). 1.0 is default; higher values result in larger support and may leave chunks at the outer edges.
    #[persist(owner = Style, ui(min = 1.0, max = 2.5, group = "footer"))]
    pub support_margin: f32,
    #[nested]
    pub meander: FlameMeander,
    #[nested]
    pub mix: FlameMix,
    #[nested]
    pub thermal: FlameThermal,
    /// Closed-form segments per ray of the wave walk: finer noise needs more
    /// (the segment grid aliases at noise frequency > ~2 with 64); 64 = default.
    #[persist]
    pub wave_segments: u32,
    #[nested]
    pub twist: FlameTwist,
    /// Line-of-sight optical thickness tau0 = sigma_t * radius: > 0 derives sigma_t as tau0 / radius so resizing the flame keeps its opacity (0 = use the raw sigma_t channel directly)
    #[persist(owner = Style, ui(min = 0.0, max = 16.0, group = "body"))]
    pub optical_depth: f32,
    #[nested]
    pub branch: FlameBranch,
    pub coefficients: FlameCoefficients,
    pub light_position_world: Vector3<f32>,
}

impl Default for FlameEffect {
    fn default() -> Self {
        let mut effect = Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            height: 1.6,
            radius: 0.6,
            sigma_t: 1.0,
            optical_depth: 0.0,
            intensity: 2.2,
            time: 0.0,
            time_scale: 1.0,
            time_offset: 0.0,
            coefficients: FlameCoefficients::default(),
            light_position_world: Vector3::new(2.0, 3.0, 2.0),
            self_shadow_strength: 0.5,
            radial_sharpness: 4.0,
            spread_gain: 0.0,
            support_margin: 1.0,
            wave_segments: crate::flame_wave::FLAME_WAVE_SEGMENTS as u32,
            color: FlameColor::default(),
            noise: FlameNoise::default(),
            warp: FlameWarp::default(),
            wind: FlameWind::default(),
            edge: FlameEdge::default(),
            envelope: FlameEnvelope::default(),
            emitter: FlameEmitter::default(),
            contour: FlameContour::default(),
            boundary: FlameBoundary::default(),
            carve: FlameCarve::default(),
            mix: FlameMix::default(),
            thermal: FlameThermal::default(),
            swirl: FlameSwirl::default(),
            twist: FlameTwist::default(),
            meander: FlameMeander::default(),
            branch: FlameBranch::default(),
        };
        refresh_flame_coefficients(&mut effect, &FlameBaked::default());
        effect
    }
}

pub fn advance_flame_time(effect: &mut FlameEffect, delta_time: f32) {
    effect.time += delta_time.max(0.0);
}

pub fn effective_sigma_t(effect: &FlameEffect) -> f32 {
    if effect.optical_depth > 0.0 {
        effect.optical_depth / effect.radius.max(MIN_FLAME_EXTENT)
    } else {
        effect.sigma_t
    }
}

pub fn flame_bounding_radius(effect: &FlameEffect) -> f32 {
    emitter_bounding_radius(&effect.emitter, effect.radius)
}

pub fn wave_segment_count(effect: &FlameEffect) -> u32 {
    effect
        .wave_segments
        .clamp(WAVE_SEGMENTS_MIN, WAVE_SEGMENTS_MAX)
}

pub fn build_flame_model_matrix(effect: &FlameEffect) -> Matrix4<f32> {
    let radius = flame_bounding_radius(effect).max(MIN_FLAME_EXTENT);
    let height = effect.height.max(MIN_FLAME_EXTENT);
    Matrix4::from_translation(effect.position)
        * Matrix4::from(effect.rotation)
        * Matrix4::from_nonuniform_scale(radius, height, radius)
}

pub fn build_flame_inverse_model_matrix(effect: &FlameEffect) -> Matrix4<f32> {
    let radius = flame_bounding_radius(effect).max(MIN_FLAME_EXTENT);
    let height = effect.height.max(MIN_FLAME_EXTENT);
    Matrix4::from_nonuniform_scale(1.0 / radius, 1.0 / height, 1.0 / radius)
        * Matrix4::from(effect.rotation.conjugate())
        * Matrix4::from_translation(-effect.position)
}

impl crate::EffectPresets for FlameEffect {
    const PRESET_NAMES: &'static [&'static str] = crate::FLAME_PRESET_NAMES;

    fn apply_preset(&mut self, name: &str) -> bool {
        crate::apply_flame_preset(self, name)
    }
}

impl crate::Placement for FlameEffect {
    fn set_placement(&mut self, time: f32, position: [f32; 3], rotation: [f32; 4]) {
        self.time = time;
        self.position = Vector3::from(position);
        crate::scene_convert::quaternion_wxyz::set(&mut self.rotation, rotation);
    }
}
