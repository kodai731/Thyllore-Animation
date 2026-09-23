use crate::flame::*;
use cgmath::{Matrix4, Quaternion, Vector3};

#[derive(
    Clone, Debug, PartialEq, thyllore_effect_derive::SceneFormat, thyllore_effect_derive::PyEffect,
)]
#[py_effect(presets = crate::FLAME_PRESET_NAMES, apply_preset = crate::apply_flame_preset)]
#[scene(record = FlameSceneRecord, tag = ParameterOwner, key = "flame", tags = PARAMETER_OWNERSHIP, snapshot = flame_parameter_snapshot, scalars = FLAME_SCALAR_PARAMS, ui = FLAME_UI_PARAMS, overwrite = overwrite_persisted_fields)]
pub struct FlameEffect {
    #[persist(owner = Frame, as = [f32; 3], get = flame_position_get, set = flame_position_set)]
    pub position: Vector3<f32>,
    #[persist(owner = Frame, as = [f32; 4], get = flame_rotation_get, set = flame_rotation_set)]
    pub rotation: Quaternion<f32>,
    #[persist(owner = Frame, ui(min = 0.05, max = 10.0, format = "%.2f", group = "body"))]
    pub height: f32,
    #[persist(owner = Frame, ui(min = 0.05, max = 10.0, format = "%.2f", group = "body"))]
    pub radius: f32,
    #[persist(owner = Style)]
    pub sigma_t: f32,
    #[persist(owner = Style, ui(min = 0.0, max = 10.0, group = "body"))]
    pub intensity: f32,
    #[persist(owner = Style, name = color_base, path = "color.base", as = [f32; 3], scalars = rgb, ui(kind = Color, label = "Base Color", min = 0.0, max = 1.0, format = "%.2f", tooltip = "Emission color at the flame base (used when blackbody is off)", group = "color"))]
    #[persist(owner = Style, name = color_tip, path = "color.tip", as = [f32; 3], scalars = rgb, ui(kind = Color, label = "Tip Color", min = 0.0, max = 1.0, format = "%.2f", tooltip = "Emission color at the flame tip (used when blackbody is off)", group = "color"))]
    #[persist(owner = Style, name = temperature_base_k, path = "color.temperature_base_k", as = f32, ui(min = 1000.0, max = 6500.0, format = "%.0f", tooltip = "Blackbody temperature at the base in kelvin"))]
    #[persist(owner = Style, name = temperature_tip_k, path = "color.temperature_tip_k", as = f32, ui(min = 1000.0, max = 6500.0, format = "%.0f", tooltip = "Blackbody temperature at the tip in kelvin"))]
    #[persist(owner = Style, name = use_blackbody, path = "color.use_blackbody", as = bool, ui(min = 0.0, max = 1.0, format = "%.0f", tooltip = "Derive the base/tip colors from the blackbody temperatures"))]
    pub color: FlameColor,
    #[persist(owner = Style, name = noise_amplitude, path = "noise.amplitude", as = f32, ui(min = 0.0, max = 3.0, group = "noise"))]
    #[persist(owner = Style, name = noise_contrast, path = "noise.contrast", as = f32, ui(min = 0.25, max = 4.0, format = "%.2f", group = "noise"))]
    #[persist(owner = Style, name = noise_frequency, path = "noise.frequency", as = f32)]
    #[persist(owner = Style, name = noise_scroll_speed, path = "noise.scroll_speed", as = f32)]
    pub noise: FlameNoise,
    #[runtime]
    pub time: f32,
    #[persist(owner = Frame, ui(min = 0.0, max = 4.0, group = "footer"))]
    pub time_scale: f32,
    #[persist(owner = Frame)]
    pub time_offset: f32,
    #[persist(owner = Style, name = warp_amp, path = "warp.amp", as = f32)]
    #[persist(owner = Style, name = warp_freq, path = "warp.freq", as = f32)]
    #[persist(owner = Style, name = rise_speed, path = "warp.rise_speed", as = f32)]
    #[persist(owner = Shape, name = taper_power, path = "warp.taper_power", as = f32)]
    #[runtime(name = warp_y_scale, path = "warp.y_scale", as = f32)]
    pub warp: FlameWarp,
    #[persist(owner = Shape, name = radius_tip_ratio, path = "edge.radius_tip_ratio", as = f32)]
    #[persist(owner = Style, name = edge_low, path = "edge.low", as = f32)]
    #[persist(owner = Style, name = edge_high, path = "edge.high", as = f32)]
    #[persist(owner = Style, name = white_boost, path = "edge.white_boost", as = f32)]
    pub edge: FlameEdge,
    #[persist(owner = Frame, name = wind_direction, as = [f32; 2], get = flame_wind_direction_get, set = flame_wind_direction_set, scalars(wind_x = "wind.direction.x", wind_z = "wind.direction.y"))]
    #[persist(owner = Frame, name = bend_amount, path = "wind.bend_amount", as = f32)]
    #[persist(owner = Frame, name = bend_power, path = "wind.bend_power", as = f32)]
    pub wind: FlameWind,
    #[persist(owner = Style)]
    pub self_shadow_strength: f32,
    #[persist(owner = Shape, name = envelope_peak, path = "envelope.peak", as = f32, default = 0.35)]
    #[persist(owner = Shape, name = envelope_base, path = "envelope.base", as = f32, default = 0.45)]
    #[persist(owner = Shape, name = envelope_tail, path = "envelope.tail", as = f32, default = 1.6)]
    pub envelope: FlameEnvelope,
    #[runtime(name = emitter_kind, path = "emitter.kind", as = u32)]
    #[runtime(name = ring_major_radius, path = "emitter.ring_major_radius", as = f32)]
    #[runtime(name = ring_angular_speed, path = "emitter.ring_angular_speed", as = f32)]
    pub emitter: FlameEmitter,
    #[runtime(name = boundary_amp, path = "boundary.amp", as = f32)]
    #[runtime(name = boundary_freq, path = "boundary.freq", as = f32)]
    #[runtime(name = boundary_speed, path = "boundary.speed", as = f32)]
    #[runtime(name = boundary_radius_ratio, path = "boundary.radius_ratio", as = f32)]
    pub boundary: FlameBoundary,
    #[persist(owner = Shape)]
    pub radial_sharpness: f32,
    #[persist(owner = Style, name = occlusion_lum_ref, path = "color.occlusion_lum_ref", as = f32)]
    #[persist(owner = Style, name = contour_wiggle_amp, path = "contour.wiggle_amp", as = f32)]
    #[persist(owner = Style, name = aniso_axis_advect, path = "contour.aniso_axis_advect", as = f32)]
    #[persist(owner = Style, name = rte_bands, path = "contour.rte_bands", as = f32)]
    #[persist(owner = Style, name = sigma_dispersion, path = "contour.sigma_dispersion", as = f32)]
    pub contour: FlameContour,
    #[persist(owner = Style, name = tip_carve_depth, path = "carve.tip.depth", as = f32)]
    #[persist(owner = Style, name = tip_carve_reach, path = "carve.tip.reach", as = f32)]
    #[persist(owner = Style, name = warp_reach, path = "warp.reach", as = f32)]
    #[runtime(name = near_fade_radius, path = "carve.near_fade_radius", as = f32)]
    #[runtime(name = carve_residual, path = "carve.residual", as = f32, ui(min = 0.0, max = 0.5, format = "%.2f", tooltip = "Translucent floor left where the noise carves the medium away. 0 = fully carved spans become hard holes — the severing moment of Burnout is only visible near 0 (debug); the product look keeps ~0.12", group = "motion"))]
    pub carve: FlameCarve,
    #[persist(owner = Style, name = swirl_gain, path = "swirl.gain", as = f32, ui(label = "Swirl", min = 0.0, max = 1.5, format = "%.2f", tooltip = "Medium swirl share: strain budget spent on azimuthal shear (0 = off; raising it thins the carve warp)", group = "noise"))]
    #[persist(owner = Style, name = swirl_speed, path = "swirl.speed", as = f32, ui(min = 0.0, max = 4.0, format = "%.2f", tooltip = "How fast the swirl layers counter-rotate against the rise (time-only: costs no strain budget)", group = "motion"))]
    pub swirl: FlameSwirl,
    /// Age-coordinate radial opening of the medium toward the tip; 0 = off.
    #[persist(owner = Style, ui(label = "Spread", min = 0.0, max = 3.0, format = "%.2f", tooltip = "Medium spread toward the tip: noise features enlarge, drift outward and dissolve as they rise (0 = rigid scroll)", group = "motion"))]
    pub spread_gain: f32,
    /// Multiplier on the shell support radius, kept matched across density
    /// support, proxy shape, and analytic integration; 1.0 = default.
    #[persist(owner = Style, ui(label = "Support", min = 1.0, max = 2.5, format = "%.2f", tooltip = "Flame density support radius: multiplier for the biweight support radius (how much extra space is allowed for carving). 1.0 is default; higher values result in larger support and may leave chunks at the outer edges.", group = "footer"))]
    pub support_margin: f32,
    #[persist(owner = Style, name = meander_amp, path = "meander.amp", as = f32, ui(label = "Meander", min = 0.0, max = 2.0, format = "%.2f", tooltip = "Horizontal meandering motion of the flame (0 = off)", group = "motion"))]
    #[persist(owner = Style, name = meander_frequency, path = "meander.frequency", as = f32, ui(min = 0.2, max = 30.0, format = "%.1f", tooltip = "Wavenumber multiplier of the meander modes: 1 = two long bends over the height, ~12 folds the column into a snake with ~4 bends (pillar reference)", group = "motion"))]
    pub meander: FlameMeander,
    #[persist(owner = Style, name = mix_lo, path = "mix.lo", as = f32, ui(min = -3.0, max = 3.0, format = "%.2f", tooltip = "Erosion carrier level (std units, carve-positive) where a parcel starts mixing with ambient air: lower mixes more of the body", group = "mix"))]
    #[persist(owner = Style, name = mix_hi, path = "mix.hi", as = f32, ui(min = -3.0, max = 4.0, format = "%.2f", tooltip = "Carrier level (std units) where a parcel counts as fully mixed (thin and cold)", group = "mix"))]
    #[persist(owner = Style, name = mix_height_gain, path = "mix.height_gain", as = f32, ui(min = 0.0, max = 2.0, format = "%.2f", tooltip = "Height ramp added to the mixing degree, gain * h^2: the plume thins and cools toward the top", group = "mix"))]
    #[persist(owner = Style, name = mix_scale, path = "mix.scale", as = f32, ui(min = 0.1, max = 2.0, format = "%.2f", tooltip = "Wavenumber of the mixing eddies relative to the low erosion octave: below 1 the mixed and unmixed regions grow larger than the carve detail", group = "mix"))]
    #[persist(owner = Style, name = mix_radial_gain, path = "mix.radial_gain", as = f32, ui(min = 0.0, max = 3.0, format = "%.2f", tooltip = "Shear-layer ramp added to the mixing degree, gain * u^2 over the normalized radius: the axis stays an unmixed bright core while the rim thins and cools", group = "mix"))]
    pub mix: FlameMix,
    #[persist(owner = Style, name = density_exp, path = "thermal.density_exp", as = f32, ui(min = 0.0, max = 4.0, format = "%.2f", tooltip = "Mass curve of a mixing parcel, (1 - m)^a: larger thins the mixed regions faster", group = "body"))]
    #[persist(owner = Style, name = temp_exp, path = "thermal.temp_exp", as = f32, ui(min = 0.0, max = 4.0, format = "%.2f", tooltip = "Temperature curve of a mixing parcel, T_cold + (T_hot - T_cold) (1 - m)^b: larger than Density Exp cools before thinning (dark red tufts remain), smaller thins before cooling", group = "body"))]
    #[persist(owner = Style, name = wien_c_k, path = "thermal.wien_c_k", as = f32, ui(label = "Wien C (K)", min = 0.0, max = 24000.0, format = "%.0f", tooltip = "Wien constant of the emissivity exp(-c/T): 24000 is physical at 0.6 um, smaller compresses the hot/cold brightness contrast like camera exposure", group = "body"))]
    pub thermal: FlameThermal,
    /// Closed-form segments per ray of the wave walk: finer noise needs more
    /// (the segment grid aliases at noise frequency > ~2 with 64); 64 = default.
    #[persist(owner = Frame)]
    pub wave_segments: u32,
    #[persist(owner = Style, name = noise_aniso_y, path = "noise.aniso_y", as = f32, ui(label = "Noise Aspect", min = 0.05, max = 1.5, format = "%.2f", tooltip = "Vertical scale of the noise cells: small = tall streaks, 1 = isotropic puffs (in the height-scaled mode)", group = "noise"))]
    #[persist(owner = Style, name = edge_outer_sharpen, path = "edge.outer_sharpen", as = f32)]
    #[persist(owner = Style, name = noise_scale_mode, path = "noise.scale_mode", as = f32)]
    #[persist(owner = Style, name = erosion_noise_gain, path = "noise.erosion_gain", as = f32)]
    #[persist(owner = Style, name = twist_gain, path = "twist.gain", as = f32, ui(label = "Twist", min = 0.0, max = 8.0, format = "%.2f", tooltip = "Azimuthal twist of the noise pattern around the axis (radians at the tip; a rotation never folds, so any amplitude is structurally safe; 0 = off)", group = "motion"))]
    #[persist(owner = Style, name = twist_speed, path = "twist.speed", as = f32, ui(min = 0.0, max = 4.0, format = "%.2f", tooltip = "Twist rotation rate scale (0 = follow Swirl Speed; > 0 gives the twist its own rate so depth and speed tune independently)", group = "motion"))]
    #[persist(owner = Style, name = burnout_gain, path = "carve.burnout_gain", as = f32, ui(label = "Burnout", min = 0.0, max = 32.0, format = "%.2f", tooltip = "Age-driven burnout of the rising material: deepens the erosion mean toward the flame top so noise troughs sever the column (base shedding) and detached tongues dissolve (0 = off; the range above ~8 is debug headroom for making the severing obvious, pair with Carve Residual 0)", group = "motion"))]
    #[persist(owner = Style, name = noise_shaping_scale, path = "noise.shaping_scale", as = f32)]
    pub twist: FlameTwist,
    /// Line-of-sight optical thickness tau0 = sigma_t * radius; > 0 derives
    /// sigma_t as optical_depth / radius, 0 = use sigma_t directly.
    #[persist(owner = Style, ui(min = 0.0, max = 16.0, format = "%.2f", tooltip = "Line-of-sight optical thickness tau0 = sigma_t * radius: > 0 derives sigma_t as tau0 / radius so resizing the flame keeps its opacity (0 = use the raw sigma_t channel directly)", group = "body"))]
    pub optical_depth: f32,
    #[persist(owner = Style, name = branch_period, path = "branch.period", as = f32, ui(min = 0.0, max = 2.0, format = "%.2f", tooltip = "Spawn period [s] of the branch elements (vortex lines that roll the medium into side tongues); raised to life/31 when the table is full, so shorter periods never shrink the tongues; 0 = off", group = "branch"))]
    #[persist(owner = Style, name = branch_life, path = "branch.life", as = f32, ui(min = 0.1, max = 6.0, format = "%.2f", tooltip = "Lifetime [s] of one element (wind out fast, hold, burn out); the spawn period is raised to life/31 when the element table is full", group = "branch"))]
    #[persist(owner = Style, name = branch_gain, path = "branch.gain", as = f32, ui(min = -8.0, max = 8.0, format = "%.2f", tooltip = "Rotation angle [rad] the core reaches at the end of winding; the medium flows along the arcs toward the tongue tip at a constant rate. Positive rolls trunk material down-out-up (cap curling inward), negative rolls it up-out-down (KH crest leaning upward). A compact rotation never folds; 0 = off", group = "branch"))]
    #[persist(owner = Style, name = branch_core_radius, path = "branch.core_radius", as = f32, ui(label = "Branch Core", min = 0.05, max = 3.0, format = "%.2f", tooltip = "Vortex core radius as a ratio of the local trunk radius: small values shear the medium into thin spirals, near 1 the whole disc turns together and tongues keep the trunk's thickness", group = "branch"))]
    #[persist(owner = Style, name = branch_core_offset, path = "branch.core_offset", as = f32, ui(min = 0.0, max = 3.0, format = "%.2f", tooltip = "Lateral position of the core at spawn as a ratio of the local trunk radius: 0 on the axis tilts the whole slab, 1 on the shear layer rolls trunk material outward as a billow", group = "branch"))]
    #[persist(owner = Style, name = branch_reach, path = "branch.reach", as = f32, ui(min = 0.5, max = 8.0, format = "%.2f", tooltip = "Compact reach of one element at the end of its life as a ratio of the local trunk radius; nothing beyond it moves, so it bounds how far tongues can extend sideways", group = "branch"))]
    #[persist(owner = Style, name = branch_spread, path = "branch.spread", as = f32, ui(min = 0.0, max = 1.0, format = "%.2f", tooltip = "Scatter of azimuth, timing jitter, left/right alternation, element size, line tilt and window shift (0 = identical elements strictly alternating in one plane)", group = "branch"))]
    #[persist(owner = Style, name = branch_spawn_height, path = "branch.spawn_height", as = f32, ui(label = "Branch Height", min = 0.0, max = 1.0, format = "%.2f", tooltip = "Center of the spawn height band (0 = base, 1 = top)", group = "branch"))]
    #[persist(owner = Style, name = branch_spawn_range, path = "branch.spawn_range", as = f32, ui(label = "Branch Height Range", min = 0.0, max = 1.0, format = "%.2f", tooltip = "Full width of the spawn height band; 1.0 with center 0.5 spawns elements over the whole trunk", group = "branch"))]
    #[persist(owner = Frame, name = branch_seed, path = "branch.seed", as = u32)]
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
