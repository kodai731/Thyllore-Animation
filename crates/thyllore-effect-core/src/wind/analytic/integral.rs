use crate::volume::{RayKnots, RayPuffs, VolumeShell};
use crate::wind::analytic::eddy::{eddy_sigma, EDDY_OCTAVE_COUNT};
use crate::wind::analytic::motion::{h_top, rotation_phase, spread_offset, streak_phase, wall_amp};
use crate::wind::analytic::puffs::build_wind_puffs;
use crate::wind::WindTornadoEffect;
use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::{biweight, mix};

// Mirror of shaders/wind/include/field.glsl and integral.glsl: the wall shell of
// crate::volume::shell plus puffs, with the streak and eddy modulation sampled on a
// ray-wide cell grid and applied linearly inside every piece.

pub const WIND_MAX_PUFFS: usize = 96;
// One cell grid spans the whole ray so a knot splitting a piece never moves a sample; the cell
// length comes from the active length (shell or puff pieces) so the budget is spent on density.
pub(crate) const MODULATION_CELLS: usize = 64;
pub(crate) const ACTIVE_CELLS_MIN: usize = 16;
const MODULATION_SAMPLE_FRACTION: f32 = 0.125;
const EMPTY_INTERVAL_EPSILON: f32 = 1e-6;
const SHADOW_RAY_T_MAX: f32 = 1e4;
const SHADOW_RADIAL_EXTENT_MARGIN: f32 = 1.25;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindShellParams {
    pub height: f32,
    pub wall_radius_base: f32,
    pub wall_radius_slope: f32,
    pub wall_width_q: f32,
    pub wall_strength: f32,
    pub top_fade: f32,
    pub sigma_t: f32,
    pub h_top: f32,
    pub spread_offset: f32,
    pub streak_order: f32,
    pub streak_twist: f32,
    pub streak_rise_speed: f32,
    pub streak_amplitude: f32,
    pub streak_phase: f32,
    pub streak_rise_time: f32,
    pub eddy_amplitude: f32,
    pub eddy_cell_theta: f32,
    pub eddy_cell_height: f32,
    pub eddy_cell_radial: f32,
    pub eddy_shear: f32,
    pub eddy_speed_spread: f32,
    pub eddy_rise_speed: f32,
    pub eddy_reseed_period: f32,
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

pub fn wind_density_at(params: &WindShellParams, local: Vector3<f32>) -> f32 {
    let shell = params.shell();
    let h = local.y / shell.height;
    let envelope = shell.envelope_height(h);
    if envelope <= 0.0 {
        return 0.0;
    }
    let q = local.x * local.x + local.z * local.z;
    let wall = shell.wall_at(q, h);

    let mut puff_term = 0.0f32;
    for puff in params.active_puffs() {
        let r = puff[3];
        if r <= 0.0 {
            continue;
        }
        let dx = local.x - puff[0];
        let dy = local.y - puff[1];
        let dz = local.z - puff[2];
        let u = (dx * dx + dy * dy + dz * dz) / (r * r);
        if u < 1.0 {
            puff_term += params.puff_strength * shell.strength * biweight(u);
        }
    }

    shell.sigma_t * (envelope * wall + puff_term)
}

/// Clamps the ray parameter interval to the wall cone frustum intersected with its
/// height slab. False when the ray misses it.
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

/// Shell knots plus the entry and exit of every puff the ray crosses, with the crossed puffs.
pub fn wind_ray_knots(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> (RayKnots, RayPuffs) {
    let mut knots = RayKnots::begin(t_near, t_far);
    params
        .shell()
        .collect_knots(origin, direction, t_near, t_far, &mut knots);
    let puffs = RayPuffs::collect(
        params.active_puffs(),
        origin,
        direction,
        t_near,
        t_far,
        &mut knots,
    );
    knots.sort();
    (knots, puffs)
}

pub fn wind_streak_sigma(params: &WindShellParams, local: Vector3<f32>) -> f32 {
    let radius_sq = params.wall_radius(local.y) * params.wall_radius(local.y);
    let rotation_phase_value = rotation_phase(
        params.time,
        params.circulation,
        radius_sq,
        params.spread_start,
        params.spread_rate,
    );
    let angle = params.streak_order * (local.z.atan2(local.x) - rotation_phase_value)
        - params.streak_twist * local.y
        + params.streak_rise_time * local.y;
    1.0 + params.streak_amplitude * angle.cos()
}

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

/// Shortest length along any ray over which the streak pattern completes one period.
fn streak_wavelength(params: &WindShellParams) -> f32 {
    let angular = params.streak_order / params.wall_radius_base.max(1e-3);
    let vertical = params.streak_rise_time - params.streak_twist;
    std::f32::consts::TAU / (angular * angular + vertical * vertical).sqrt().max(1e-3)
}

fn piece_is_active(
    shell: &VolumeShell,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    puffs: &RayPuffs,
    s0: f32,
    s1: f32,
) -> bool {
    shell.piece_holds(origin, direction, s0, s1) || puffs.piece_holds(s0, s1)
}

/// Length of the ray inside the shell support or a puff: continuous in the ray, so the cell
/// length derived from it never jumps when a knot appears.
pub fn wind_active_length(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    knots: &RayKnots,
    puffs: &RayPuffs,
) -> f32 {
    let shell = params.shell();
    knots
        .pieces()
        .filter(|&(s0, s1)| piece_is_active(&shell, origin, direction, puffs, s0, s1))
        .map(|(s0, s1)| s1 - s0)
        .sum()
}

/// Cell length along the ray: a fraction of the finest active modulation feature, bounded so the
/// active length holds between ACTIVE_CELLS_MIN and MODULATION_CELLS cells.
pub fn wind_modulation_step(
    params: &WindShellParams,
    direction: Vector3<f32>,
    active_length: f32,
) -> f32 {
    let span = active_length.max(EMPTY_INTERVAL_EPSILON);
    let mut finest_feature = span * direction.magnitude();
    if params.streak_amplitude > 0.0 {
        finest_feature = finest_feature.min(0.5 * streak_wavelength(params));
    }
    if params.eddy_amplitude > 0.0 {
        let finest_octave_scale = 2f32.powi(EDDY_OCTAVE_COUNT as i32 - 1);
        let finest_cell = params
            .eddy_cell_height
            .min(params.eddy_cell_theta.min(params.eddy_cell_radial));
        finest_feature = finest_feature.min(finest_cell / finest_octave_scale);
    }
    (MODULATION_SAMPLE_FRACTION * finest_feature / direction.magnitude()).clamp(
        span / MODULATION_CELLS as f32,
        span / ACTIVE_CELLS_MIN as f32,
    )
}

/// Optical depth of the puffs whose entry/exit knots enclose the piece [s0, s1].
pub fn wind_puff_piece_optical_depth(
    params: &WindShellParams,
    puffs: &RayPuffs,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    s0: f32,
    s1: f32,
) -> f32 {
    let integral = puffs.piece_integral(params.active_puffs(), origin, direction, s0, s1);
    (params.sigma_t * params.puff_strength * params.wall_strength * integral).max(0.0)
}

/// Modulation at both nodes of a cell; a node shared with the previous cell is not re-evaluated.
#[derive(Clone, Copy)]
struct CellModulation {
    cell: Option<i64>,
    a: f32,
    b: f32,
}

impl CellModulation {
    fn none() -> Self {
        Self {
            cell: None,
            a: 1.0,
            b: 1.0,
        }
    }

    fn advance(
        self,
        params: &WindShellParams,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        t_near: f32,
        step: f32,
        cell: i64,
    ) -> Self {
        if self.cell == Some(cell) {
            return self;
        }
        let cell_start = t_near + cell as f32 * step;
        let a = if self.cell == Some(cell - 1) {
            self.b
        } else {
            wind_modulation_at(params, origin + direction * cell_start, direction * step)
        };
        let b = wind_modulation_at(
            params,
            origin + direction * (cell_start + step),
            direction * step,
        );
        Self {
            cell: Some(cell),
            a,
            b,
        }
    }
}

/// Pieces are walked in ray order and each is cut by the cells of the ray-wide grid it overlaps.
pub fn wind_optical_depth(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> f32 {
    if t_far <= t_near {
        return 0.0;
    }
    let shell = params.shell();
    let (knots, puffs) = wind_ray_knots(params, origin, direction, t_near, t_far);
    let active_length = wind_active_length(params, origin, direction, &knots, &puffs);
    if active_length <= EMPTY_INTERVAL_EPSILON {
        return 0.0;
    }
    let step = wind_modulation_step(params, direction, active_length);

    let mut total = 0.0f32;
    let mut modulation = CellModulation::none();
    for (piece_start, piece_end) in knots.pieces() {
        if !piece_is_active(&shell, origin, direction, &puffs, piece_start, piece_end) {
            continue;
        }
        let cell_first = ((piece_start - t_near) / step).floor() as i64;
        let cell_last = (((piece_end - t_near) / step).floor() as i64)
            .min(cell_first + MODULATION_CELLS as i64);
        for cell in cell_first..=cell_last {
            let cell_start = t_near + cell as f32 * step;
            let s0 = piece_start.max(cell_start);
            let s1 = piece_end.min(cell_start + step);
            if s1 <= s0 {
                continue;
            }
            modulation = modulation.advance(params, origin, direction, t_near, step, cell);
            let modulation_0 = mix(modulation.a, modulation.b, (s0 - cell_start) / step);
            let modulation_1 = mix(modulation.a, modulation.b, (s1 - cell_start) / step);
            total += shell.piece_optical_depth_modulated(
                origin,
                direction,
                s0,
                s1,
                (modulation_0, modulation_1),
            ) + wind_puff_piece_optical_depth(params, &puffs, origin, direction, s0, s1);
        }
    }
    total
}

/// Wall + envelope optical depth along the ray (shadow rays drop streak, eddy and puffs).
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
pub fn wind_optical_depth_toward(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
) -> f32 {
    params
        .shell()
        .optical_depth_toward(origin, direction, SHADOW_RAY_T_MAX)
}
