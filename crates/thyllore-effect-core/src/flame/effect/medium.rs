use crate::flame::*;

/// Mixing of the medium with ambient air along the erosion carrier.
#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner, owner = Style, group = "mix")]
pub struct FlameMix {
    /// Erosion carrier level (std units, carve-positive) where a parcel starts mixing with ambient air: lower mixes more of the body
    #[persist(ui(min = -3.0, max = 3.0))]
    pub lo: f32,
    /// Carrier level (std units) where a parcel counts as fully mixed (thin and cold)
    #[persist(ui(min = -3.0, max = 4.0))]
    pub hi: f32,
    /// Height ramp added to the mixing degree, gain * h^2: the plume thins and cools toward the top
    #[persist(ui(min = 0.0, max = 2.0))]
    pub height_gain: f32,
    /// Wavenumber of the mixing eddies relative to the low erosion octave: below 1 the mixed and unmixed regions grow larger than the carve detail
    #[persist(ui(min = 0.1, max = 2.0))]
    pub scale: f32,
    /// Shear-layer ramp added to the mixing degree, gain * u^2 over the normalized radius: the axis stays an unmixed bright core while the rim thins and cools
    #[persist(ui(min = 0.0, max = 3.0))]
    pub radial_gain: f32,
}

impl Default for FlameMix {
    fn default() -> Self {
        Self {
            lo: 0.0,
            hi: 2.0,
            height_gain: 0.0,
            scale: 1.0,
            radial_gain: 0.0,
        }
    }
}

/// Density and temperature response of a parcel to its mixing degree m.
#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner, owner = Style, group = "body")]
pub struct FlameThermal {
    /// Mass curve of a mixing parcel, (1 - m)^a: larger thins the mixed regions faster
    #[persist(ui(min = 0.0, max = 4.0))]
    pub density_exp: f32,
    /// Temperature curve of a mixing parcel, T_cold + (T_hot - T_cold) (1 - m)^b: larger than Density Exp cools before thinning (dark red tufts remain), smaller thins before cooling
    #[persist(ui(min = 0.0, max = 4.0))]
    pub temp_exp: f32,
    /// Wien constant of the emissivity exp(-c/T): 24000 is physical at 0.6 um, smaller compresses the hot/cold brightness contrast like camera exposure
    #[persist(ui(label = "Wien C (K)", min = 0.0, max = 24000.0, format = "%.0f"))]
    pub wien_c_k: f32,
}

impl Default for FlameThermal {
    fn default() -> Self {
        Self {
            density_exp: 1.0,
            temp_exp: 1.0,
            wien_c_k: 12000.0,
        }
    }
}

/// Carve deepening toward the flame's own luminous tip:
/// relative carve scales as 1 + depth * exp(-mu / reach).
#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner, owner = Style)]
pub struct FlameTipCarve {
    /// Asymptotic extra depth kappa; 0 = uniform depth.
    #[persist]
    pub depth: f32,
    /// Reach mu0 in remaining-luminous-fraction units (scale-free).
    #[persist]
    pub reach: f32,
}

impl Default for FlameTipCarve {
    fn default() -> Self {
        Self {
            depth: 1.0,
            reach: 0.2,
        }
    }
}

/// Where and how deep the turbulence carves the soot away.
#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner, owner = Style, group = "motion")]
pub struct FlameCarve {
    #[runtime]
    pub near_fade_radius: f32,
    /// Translucent floor left where the noise carves the medium away. 0 = fully carved spans become hard holes — the severing moment of Burnout is only visible near 0 (debug); the product look keeps ~0.12
    #[runtime(ui(min = 0.0, max = 0.5))]
    pub residual: f32,
    #[nested]
    pub tip: FlameTipCarve,
    /// Age-driven burnout of the rising material: deepens the erosion mean toward the flame top so noise troughs sever the column (base shedding) and detached tongues dissolve (0 = off; the range above ~8 is debug headroom for making the severing obvious, pair with Carve Residual 0)
    #[persist(ui(label = "Burnout", min = 0.0, max = 32.0))]
    pub burnout_gain: f32,
}

impl Default for FlameCarve {
    fn default() -> Self {
        Self {
            near_fade_radius: 0.0,
            residual: 0.12,
            tip: FlameTipCarve::default(),
            burnout_gain: 0.0,
        }
    }
}

pub fn build_mix_params(mix: &FlameMix, low_carrier_std: f32) -> FlameMixParams {
    FlameMixParams {
        lo: mix.lo,
        hi: mix.hi.max(mix.lo + 1e-3),
        inv_carrier_std: 1.0 / low_carrier_std.max(1e-6),
        height_gain: mix.height_gain.max(0.0),
        scale: mix.scale.max(1e-3),
        radial_gain: mix.radial_gain.max(0.0),
        _padding: [0.0; 2],
    }
}

pub fn build_thermal_params(thermal: &FlameThermal, color: &FlameColor) -> FlameThermalParams {
    FlameThermalParams {
        density_exp: thermal.density_exp.max(0.0),
        temp_exp: thermal.temp_exp.max(0.0),
        temp_hot_k: color.temperature_base_k.max(1.0),
        temp_cold_k: color.temperature_tip_k.max(1.0),
        wien_ck: thermal.wien_c_k.max(0.0),
        _padding: [0.0; 3],
    }
}

pub fn build_near_fade_params(carve: &FlameCarve, edge_window: (f32, f32)) -> FlameNearFadeParams {
    FlameNearFadeParams {
        radius: carve.near_fade_radius,
        carve_residual: carve.residual,
        edge_low: edge_window.0,
        edge_high: edge_window.1,
    }
}

pub fn build_tip_carve_params(
    carve: &FlameCarve,
    coefficients: &FlameCoefficients,
) -> FlameTipCarveParams {
    let primitive = thyllore_math_core::ChebyshevSeries::new(
        coefficients
            .height_primitive
            .iter()
            .flatten()
            .copied()
            .collect(),
        (0.0, 1.0),
    );
    let (at_base, at_top) = thyllore_math_core::chebyshev_endpoint_values(&primitive);
    let total = at_top - at_base;
    let inv_total = if total.abs() > 1e-6 { 1.0 / total } else { 0.0 };
    FlameTipCarveParams {
        depth: carve.tip.depth,
        inv_reach: 1.0 / carve.tip.reach.max(1e-3),
        primitive_top: at_top,
        inv_primitive_range: inv_total,
    }
}
