#[cfg(test)]
use crate::volume::{
    cell_modulated_optical_depth, modulation_step, shell_and_puff_density, shell_and_puff_knots,
    RayKnots, RayMedium, RayPuffs,
};
use crate::volume::{StreakModulation, VolumeShell};
#[cfg(test)]
use crate::wind::analytic::eddy::{eddy_sigma, EDDY_OCTAVE_COUNT};
#[cfg(test)]
use crate::wind::analytic::motion::rotation_phase;
use crate::wind::analytic::motion::{h_top, spread_offset, streak_phase, wall_amp};
use crate::wind::analytic::puffs::build_wind_puffs;
use crate::wind::WindTornadoEffect;
#[cfg(test)]
use cgmath::Vector3;

// Mirror of shaders/wind/include/field.slang and integral.slang: the shell, puffs, streak and
// cell modulation of crate::volume with the tornado's parameters filled in.

pub const WIND_MAX_PUFFS: usize = 96;
#[cfg(test)]
const SHADOW_RAY_T_MAX: f32 = 1e4;
const SHADOW_RADIAL_EXTENT_MARGIN: f32 = 1.25;

#[derive(Clone, Copy, Debug, PartialEq, thyllore_effect_derive::UboPack)]
#[ubo(target = crate::WindUBO)]
pub struct WindShellParams {
    #[ubo("shape.x")]
    pub height: f32,
    #[ubo("shape.y")]
    pub wall_radius_base: f32,
    #[ubo("shape.z")]
    pub wall_radius_slope: f32,
    #[ubo("shape.w")]
    pub wall_width_q: f32,
    #[ubo("wall.x")]
    pub wall_strength: f32,
    #[ubo("wall.y")]
    pub top_fade: f32,
    #[ubo("optics.x")]
    pub sigma_t: f32,
    #[ubo("optics.w")]
    pub h_top: f32,
    #[ubo("albedo.w")]
    pub spread_offset: f32,
    #[ubo("streak.x")]
    pub streak_order: f32,
    #[ubo("streak.y")]
    pub streak_twist: f32,
    #[ubo("streak.z")]
    pub streak_rise_speed: f32,
    #[ubo("streak.w")]
    pub streak_amplitude: f32,
    #[ubo("streak2.x")]
    pub streak_phase: f32,
    #[ubo("streak2.y")]
    pub streak_rise_time: f32,
    #[ubo("eddy.x")]
    pub eddy_amplitude: f32,
    #[ubo("eddy.y")]
    pub eddy_cell_theta: f32,
    #[ubo("eddy.z")]
    pub eddy_cell_height: f32,
    #[ubo("eddy.w")]
    pub eddy_cell_radial: f32,
    #[ubo("eddy2.x")]
    pub eddy_shear: f32,
    pub eddy_speed_spread: f32,
    #[ubo("eddy2.y")]
    pub eddy_rise_speed: f32,
    #[ubo("eddy2.z")]
    pub eddy_reseed_period: f32,
    #[ubo("eddy2.w")]
    pub eddy_erosion: f32,
    pub time: f32,
    pub circulation: f32,
    pub spread_start: f32,
    pub spread_rate: f32,
    pub puff_strength: f32,
    pub puff_count: usize,
    pub puffs: [[f32; 4]; WIND_MAX_PUFFS],
}

impl WindShellParams {
    pub fn from_effect(effect: &WindTornadoEffect) -> Self {
        let t = effect.time;
        let h_top_value = h_top(t, effect.rise_initial_height, effect.rise_duration);
        let spread_offset_value = spread_offset(t, effect.spread_start, effect.spread_rate);
        let wall_strength = wall_amp(
            t,
            effect.wall_strength,
            effect.dissipate_start,
            effect.dissipate_time,
        );
        let streak_phase_value = streak_phase(
            t,
            effect.circulation,
            effect.wall_radius_base,
            effect.spread_start,
            effect.spread_rate,
        );
        let mut params = Self {
            height: effect.column_height.max(1e-3),
            wall_radius_base: effect.wall_radius_base,
            wall_radius_slope: effect.wall_radius_top - effect.wall_radius_base,
            wall_width_q: effect.wall_width_q.max(1e-4),
            wall_strength,
            top_fade: effect.top_fade.clamp(1e-3, 1.0),
            sigma_t: effect.density,
            h_top: h_top_value.max(1e-3),
            spread_offset: spread_offset_value,
            streak_order: effect.streak_order,
            streak_twist: effect.streak_twist,
            streak_rise_speed: effect.streak_rise_speed,
            streak_amplitude: effect.streak_amplitude.max(0.0),
            streak_phase: streak_phase_value,
            streak_rise_time: effect.streak_rise_speed * t,
            eddy_amplitude: effect.eddy_amplitude.max(0.0),
            eddy_cell_theta: effect.eddy_cell_theta.max(1e-3),
            eddy_cell_height: effect.eddy_cell_height.max(1e-3),
            eddy_cell_radial: effect.eddy_cell_radial.max(1e-3),
            eddy_shear: effect.eddy_shear.clamp(0.0, 1.0),
            eddy_speed_spread: effect.eddy_speed_spread,
            eddy_rise_speed: effect.eddy_rise_speed,
            eddy_reseed_period: effect.eddy_reseed_period.max(1e-3),
            eddy_erosion: effect.eddy_erosion.clamp(0.0, 0.95),
            time: t,
            circulation: effect.circulation,
            spread_start: effect.spread_start,
            spread_rate: effect.spread_rate,
            puff_strength: effect.puff_strength,
            puff_count: 0,
            puffs: [[0.0; 4]; WIND_MAX_PUFFS],
        };

        let (puffs, puff_count) = build_wind_puffs(effect, &params);
        params.puffs = puffs;
        params.puff_count = puff_count;
        params
    }

    pub fn shell(&self) -> VolumeShell {
        VolumeShell {
            height: self.height,
            radius_base: self.wall_radius_base,
            radius_slope: self.wall_radius_slope,
            radius_offset_q: self.spread_offset,
            width_q: self.wall_width_q,
            strength: self.wall_strength,
            h_top: self.h_top,
            top_fade: self.top_fade,
            sigma_t: self.sigma_t,
        }
    }

    pub(crate) fn wall_radius(&self, h: f32) -> f32 {
        self.shell().wall_radius(h)
    }

    pub(crate) fn wall_radius_sq(&self, h: f32) -> f32 {
        self.shell().wall_radius_sq(h)
    }

    pub fn active_puffs(&self) -> &[[f32; 4]] {
        &self.puffs[..self.puff_count]
    }

    pub fn streak(&self) -> StreakModulation {
        StreakModulation {
            order: self.streak_order,
            twist: self.streak_twist,
            rise_time: self.streak_rise_time,
            amplitude: self.streak_amplitude,
        }
    }

    #[cfg(test)]
    fn puff_sigma(&self) -> f32 {
        self.sigma_t * self.puff_strength * self.wall_strength
    }

    /// Phase of the wall rotation at the radius of the wall at height `y`.
    #[cfg(test)]
    fn wall_rotation_phase(&self, y: f32) -> f32 {
        let radius_sq = self.wall_radius(y) * self.wall_radius(y);
        rotation_phase(
            self.time,
            self.circulation,
            radius_sq,
            self.spread_start,
            self.spread_rate,
        )
    }
}

#[cfg(test)]
pub struct WindRay {
    shell: VolumeShell,
    puffs: RayPuffs,
}
#[cfg(test)]
impl RayMedium for WindShellParams {
    type Ray = WindRay;

    fn ray_pieces(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        t_near: f32,
        t_far: f32,
    ) -> (RayKnots, WindRay) {
        let (knots, puffs) = shell_and_puff_knots(
            &self.shell(),
            self.active_puffs(),
            origin,
            direction,
            t_near,
            t_far,
        );
        let ray = WindRay {
            shell: self.shell(),
            puffs,
        };
        (knots, ray)
    }

    fn piece_is_active(
        &self,
        ray: &WindRay,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        s0: f32,
        s1: f32,
    ) -> bool {
        ray.shell.piece_holds(origin, direction, s0, s1) || ray.puffs.piece_holds(s0, s1)
    }

    fn modulation_at(&self, point: Vector3<f32>, step_ahead: Vector3<f32>) -> f32 {
        wind_modulation_at(self, point, step_ahead)
    }

    fn modulation_step(&self, direction: Vector3<f32>, active_length: f32) -> f32 {
        wind_modulation_step(self, direction, active_length)
    }

    fn piece_optical_depth(
        &self,
        ray: &WindRay,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        s0: f32,
        s1: f32,
        modulation: (f32, f32),
    ) -> f32 {
        ray.shell
            .piece_optical_depth_modulated(origin, direction, s0, s1, modulation)
            + ray.puffs.piece_optical_depth(
                self.active_puffs(),
                self.puff_sigma(),
                origin,
                direction,
                s0,
                s1,
            )
    }
}

/// Half width of the shadow volume's radius axis around the wall: covers the shell at its
/// thickest (smallest radius) and every puff, with a margin for linear filtering.
pub fn wind_shadow_radial_extent(params: &WindShellParams) -> f32 {
    let thinnest_radius_sq = params
        .wall_radius_sq(0.0)
        .min(params.wall_radius_sq(params.h_top))
        .max(0.0);
    let shell_half_width =
        (thinnest_radius_sq + params.wall_width_q).sqrt() - thinnest_radius_sq.sqrt();

    let mut extent = shell_half_width;
    for puff in params.active_puffs() {
        let wall_radius = params
            .wall_radius_sq(puff[1] / params.height)
            .max(0.0)
            .sqrt();
        let puff_reach =
            ((puff[0] * puff[0] + puff[2] * puff[2]).sqrt() - wall_radius).abs() + puff[3];
        extent = extent.max(puff_reach);
    }
    (extent * SHADOW_RADIAL_EXTENT_MARGIN).max(1e-3)
}
#[cfg(test)]
pub fn wind_density_at(params: &WindShellParams, local: Vector3<f32>) -> f32 {
    shell_and_puff_density(
        &params.shell(),
        params.active_puffs(),
        params.puff_strength,
        local,
    )
}

/// Clamps the ray parameter interval to the wall cone frustum intersected with its
/// height slab. False when the ray misses it.
#[cfg(test)]
pub fn clamp_ray_to_wind_cone(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: &mut f32,
    t_far: &mut f32,
) -> bool {
    let Some(interval) = params
        .shell()
        .clamp_ray_to_cone(origin, direction, (*t_near, *t_far))
    else {
        return false;
    };
    *t_near = interval.0;
    *t_far = interval.1;
    true
}

#[cfg(test)]
pub fn wind_streak_sigma(params: &WindShellParams, local: Vector3<f32>) -> f32 {
    params
        .streak()
        .sigma(local, params.wall_rotation_phase(local.y))
}

#[cfg(test)]
pub fn wind_modulation_at(
    params: &WindShellParams,
    local: Vector3<f32>,
    step_ahead: Vector3<f32>,
) -> f32 {
    let mut modulation = 1.0;
    if params.streak_amplitude > 0.0 {
        modulation *= wind_streak_sigma(params, local);
    }
    if params.eddy_amplitude > 0.0 {
        modulation *= eddy_sigma(
            params,
            [local.x, local.y, local.z],
            [step_ahead.x, step_ahead.y, step_ahead.z],
        );
    }
    modulation
}

/// Finest active modulation feature in world units: half a streak period or the finest eddy octave cell.
#[cfg(test)]
fn wind_finest_feature(params: &WindShellParams) -> Option<f32> {
    let streak = (params.streak_amplitude > 0.0)
        .then(|| 0.5 * params.streak().wavelength(params.wall_radius_base));
    let eddy = (params.eddy_amplitude > 0.0).then(|| {
        let finest_octave_scale = 2f32.powi(EDDY_OCTAVE_COUNT as i32 - 1);
        let finest_cell = params
            .eddy_cell_height
            .min(params.eddy_cell_theta.min(params.eddy_cell_radial));
        finest_cell / finest_octave_scale
    });
    match (streak, eddy) {
        (Some(streak), Some(eddy)) => Some(streak.min(eddy)),
        (feature, None) | (None, feature) => feature,
    }
}

#[cfg(test)]
pub fn wind_modulation_step(
    params: &WindShellParams,
    direction: Vector3<f32>,
    active_length: f32,
) -> f32 {
    modulation_step(direction, active_length, wind_finest_feature(params))
}

#[cfg(test)]
pub fn wind_optical_depth(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> f32 {
    cell_modulated_optical_depth(params, origin, direction, t_near, t_far)
}

/// Wall + envelope optical depth along the ray (shadow rays drop streak, eddy and puffs).
#[cfg(test)]
pub fn wind_shadow_optical_depth(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> f32 {
    params
        .shell()
        .optical_depth(origin, direction, t_near, t_far)
}

/// Shadow optical depth from `origin` toward `direction` up to the cone boundary.
#[cfg(test)]
pub fn wind_optical_depth_toward(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
) -> f32 {
    params
        .shell()
        .optical_depth_toward(origin, direction, SHADOW_RAY_T_MAX)
}
