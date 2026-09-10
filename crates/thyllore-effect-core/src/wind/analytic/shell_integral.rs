use crate::wind::analytic::eddy::eddy_sigma;
use crate::wind::analytic::motion::{h_top, rotation_phase, spread_offset, streak_phase, wall_amp};
use crate::wind::analytic::puffs::build_wind_puffs;
use crate::wind::WindTornadoEffect;
use cgmath::{InnerSpace, Vector3};

// Mirror of shaders/wind/include/shell_field.glsl and shell_integral.glsl.
//
// The density is a compact-support polynomial shell in q = x^2 + z^2 plus puffs:
//   wall: B((q - P(h)) / W),  P(h) = (base + slope * h)^2
// with B(u) = (1 - u^2)^2 on |u| < 1, scaled by the height envelope E(h).
// Along a ray q and h are quadratic and linear in the ray parameter, so every
// term is a polynomial and the optical depth of a piece between two knots is
// an exact power-rule integral in the piece-local variable sigma in [0, 1].
// Puffs add their own entry/exit knots and are integrated only on the pieces
// between them; shadow rays keep the wall and envelope alone.

pub const WIND_MAX_KNOTS: usize = 56;
pub const WIND_MAX_PUFFS: usize = 96;
pub const WIND_PUFFS_PER_RAY: usize = 20;
pub(crate) const POLY_TERMS: usize = 16;
// Fixed so the node set is a continuous function of the ray (no seams where a count would change).
pub(crate) const EDDY_SPLITS: usize = 8;
const LINEAR_COEFFICIENT_EPSILON: f32 = 1e-7;
const EMPTY_INTERVAL_EPSILON: f32 = 1e-6;
const SHADOW_RAY_T_MAX: f32 = 1e4;
pub const WIND_ZENITH_DIRECTION: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);

type Poly = [f32; POLY_TERMS];

/// Puffs a ray enters, with their entry and exit ray parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindRayPuffs {
    pub count: usize,
    pub index: [usize; WIND_PUFFS_PER_RAY],
    pub enter: [f32; WIND_PUFFS_PER_RAY],
    pub exit: [f32; WIND_PUFFS_PER_RAY],
}

impl Default for WindRayPuffs {
    fn default() -> Self {
        Self {
            count: 0,
            index: [0; WIND_PUFFS_PER_RAY],
            enter: [0.0; WIND_PUFFS_PER_RAY],
            exit: [0.0; WIND_PUFFS_PER_RAY],
        }
    }
}

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

    pub(crate) fn wall_radius(&self, h: f32) -> f32 {
        self.wall_radius_base + self.wall_radius_slope * h
    }

    pub(crate) fn wall_radius_sq(&self, h: f32) -> f32 {
        let radius = self.wall_radius(h);
        radius * radius + self.spread_offset
    }

    fn fade_start(&self) -> f32 {
        1.0 - self.top_fade
    }
}

pub fn wind_envelope_radius(params: &WindShellParams, h: f32) -> f32 {
    params.wall_radius_sq(h).max(0.0).sqrt() + params.wall_width_q.sqrt()
}

pub fn wind_envelope_height(params: &WindShellParams, h: f32) -> f32 {
    if h < 0.0 || h > params.h_top {
        return 0.0;
    }
    let normalized_height = h / params.h_top;
    let fade_start = params.fade_start();
    if normalized_height <= fade_start {
        return 1.0;
    }
    let v = (normalized_height - fade_start) / params.top_fade;
    1.0 - v * v * (3.0 - 2.0 * v)
}

fn biweight(u: f32) -> f32 {
    let inside = (1.0 - u * u).max(0.0);
    inside * inside
}

pub fn wind_density_at(params: &WindShellParams, local: Vector3<f32>) -> f32 {
    let h = local.y / params.height;
    let envelope = wind_envelope_height(params, h);
    if envelope <= 0.0 {
        return 0.0;
    }
    let q = local.x * local.x + local.z * local.z;

    let wall =
        params.wall_strength * biweight((q - params.wall_radius_sq(h)) / params.wall_width_q);

    let mut puff_term = 0.0f32;
    for i in 0..params.puff_count {
        let puff = &params.puffs[i];
        let cx = puff[0];
        let cy = puff[1];
        let cz = puff[2];
        let r = puff[3];
        if r <= 0.0 {
            continue;
        }
        let dx = local.x - cx;
        let dy = local.y - cy;
        let dz = local.z - cz;
        let u = (dx * dx + dy * dy + dz * dz) / (r * r);
        if u < 1.0 {
            puff_term += params.puff_strength * params.wall_strength * biweight(u);
        }
    }

    params.sigma_t * (envelope * wall + puff_term)
}

fn clamp_ray_to_cone_frustum(
    radius_base: f32,
    radius_top: f32,
    top_y: f32,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    bounds: (f32, f32),
) -> Option<(f32, f32)> {
    let (mut t_near, mut t_far) = bounds;
    let slope_per_unit_y = (radius_top - radius_base) / top_y;
    let m = radius_base + slope_per_unit_y * origin.y;
    let n = slope_per_unit_y * direction.y;
    let a = direction.x * direction.x + direction.z * direction.z - n * n;
    let b = 2.0 * (origin.x * direction.x + origin.z * direction.z - m * n);
    let c = origin.x * origin.x + origin.z * origin.z - m * m;

    if a.abs() < LINEAR_COEFFICIENT_EPSILON {
        if b.abs() < LINEAR_COEFFICIENT_EPSILON {
            if c > 0.0 {
                return None;
            }
        } else {
            let t_root = -c / b;
            if b > 0.0 {
                t_far = t_far.min(t_root);
            } else {
                t_near = t_near.max(t_root);
            }
        }
    } else {
        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            if a > 0.0 {
                return None;
            }
        } else if a > 0.0 {
            let sqrt_discriminant = discriminant.sqrt();
            let t0 = (-b - sqrt_discriminant) / (2.0 * a);
            let t1 = (-b + sqrt_discriminant) / (2.0 * a);
            t_near = t_near.max(t0.min(t1));
            t_far = t_far.min(t0.max(t1));
        }
    }

    if direction.y.abs() < LINEAR_COEFFICIENT_EPSILON {
        if origin.y < 0.0 || origin.y > top_y {
            return None;
        }
    } else {
        let t_y0 = -origin.y / direction.y;
        let t_y1 = (top_y - origin.y) / direction.y;
        t_near = t_near.max(t_y0.min(t_y1));
        t_far = t_far.min(t_y0.max(t_y1));
    }

    (t_near <= t_far).then_some((t_near, t_far))
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
    let Some(interval) = clamp_ray_to_cone_frustum(
        wind_envelope_radius(params, 0.0),
        wind_envelope_radius(params, params.h_top),
        params.h_top * params.height,
        origin,
        direction,
        (*t_near, *t_far),
    ) else {
        return false;
    };
    *t_near = interval.0;
    *t_far = interval.1;
    true
}

fn push_knot(knots: &mut [f32; WIND_MAX_KNOTS], count: &mut usize, t: f32, lo: f32, hi: f32) {
    if t <= lo || t >= hi || *count >= WIND_MAX_KNOTS {
        return;
    }
    knots[*count] = t;
    *count += 1;
}

fn push_quadratic_roots(
    a: f32,
    b: f32,
    c: f32,
    lo: f32,
    hi: f32,
    knots: &mut [f32; WIND_MAX_KNOTS],
    count: &mut usize,
) {
    if a.abs() < LINEAR_COEFFICIENT_EPSILON {
        if b.abs() >= LINEAR_COEFFICIENT_EPSILON {
            push_knot(knots, count, -c / b, lo, hi);
        }
        return;
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return;
    }
    let sqrt_discriminant = discriminant.sqrt();
    push_knot(knots, count, (-b - sqrt_discriminant) / (2.0 * a), lo, hi);
    push_knot(knots, count, (-b + sqrt_discriminant) / (2.0 * a), lo, hi);
}

fn sort_knots(knots: &mut [f32; WIND_MAX_KNOTS], count: usize) {
    for i in 1..count {
        let value = knots[i];
        let mut j = i;
        while j > 0 && knots[j - 1] > value {
            knots[j] = knots[j - 1];
            j -= 1;
        }
        knots[j] = value;
    }
}

fn collect_shell_knots(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> ([f32; WIND_MAX_KNOTS], usize) {
    let mut knots = [0.0f32; WIND_MAX_KNOTS];
    let mut count = 2usize;
    knots[0] = t_near;
    knots[1] = t_far;

    let q_a = direction.x * direction.x + direction.z * direction.z;
    let q_b = 2.0 * (origin.x * direction.x + origin.z * direction.z);
    let q_c = origin.x * origin.x + origin.z * origin.z;

    let inv_height = 1.0 / params.height;
    let radius_0 = params.wall_radius(origin.y * inv_height);
    let radius_1 = params.wall_radius_slope * direction.y * inv_height;
    let delta_a = q_a - radius_1 * radius_1;
    let delta_b = q_b - 2.0 * radius_0 * radius_1;
    let delta_c = q_c - radius_0 * radius_0 - params.spread_offset;
    for boundary in [params.wall_width_q, -params.wall_width_q] {
        push_quadratic_roots(
            delta_a,
            delta_b,
            delta_c - boundary,
            t_near,
            t_far,
            &mut knots,
            &mut count,
        );
    }

    if direction.y.abs() >= LINEAR_COEFFICIENT_EPSILON {
        let fade_y = params.fade_start() * params.h_top * params.height;
        push_knot(
            &mut knots,
            &mut count,
            (fade_y - origin.y) / direction.y,
            t_near,
            t_far,
        );
    }
    (knots, count)
}

/// Ray parameters where a shell support boundary or the envelope break is crossed,
/// sorted ascending and bracketed by `t_near` / `t_far`.
pub fn wind_shell_knots(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> ([f32; WIND_MAX_KNOTS], usize) {
    let (mut knots, count) = collect_shell_knots(params, origin, direction, t_near, t_far);
    sort_knots(&mut knots, count);
    (knots, count)
}

/// Shell knots plus the entry and exit of every puff the ray crosses, with the crossed puffs.
pub fn wind_ray_knots(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> ([f32; WIND_MAX_KNOTS], usize, WindRayPuffs) {
    let (mut knots, mut count) = collect_shell_knots(params, origin, direction, t_near, t_far);

    let mut puffs = WindRayPuffs::default();
    let a = direction.dot(direction);
    for i in 0..params.puff_count {
        if puffs.count >= WIND_PUFFS_PER_RAY {
            break;
        }
        let puff = &params.puffs[i];
        let r = puff[3];
        if r <= 0.0 {
            continue;
        }
        let dx = origin - Vector3::new(puff[0], puff[1], puff[2]);
        let b = 2.0 * dx.dot(direction);
        let c = dx.dot(dx) - r * r;
        let discriminant = b * b - 4.0 * a * c;
        if discriminant <= 0.0 {
            continue;
        }
        let sqrt_discriminant = discriminant.sqrt();
        let t0 = (-b - sqrt_discriminant) / (2.0 * a);
        let t1 = (-b + sqrt_discriminant) / (2.0 * a);
        if t1 <= t_near || t0 >= t_far {
            continue;
        }
        push_knot(&mut knots, &mut count, t0, t_near, t_far);
        push_knot(&mut knots, &mut count, t1, t_near, t_far);
        puffs.index[puffs.count] = i;
        puffs.enter[puffs.count] = t0;
        puffs.exit[puffs.count] = t1;
        puffs.count += 1;
    }

    sort_knots(&mut knots, count);
    (knots, count, puffs)
}

fn poly_mul(a: &Poly, b: &Poly) -> Poly {
    let mut product = [0.0f32; POLY_TERMS];
    for (i, &ai) in a.iter().enumerate() {
        if ai == 0.0 {
            continue;
        }
        for (j, &bj) in b.iter().enumerate() {
            if i + j >= POLY_TERMS {
                break;
            }
            product[i + j] += ai * bj;
        }
    }
    product
}

fn poly_from_quadratic(c0: f32, c1: f32, c2: f32) -> Poly {
    let mut poly = [0.0f32; POLY_TERMS];
    poly[0] = c0;
    poly[1] = c1;
    poly[2] = c2;
    poly
}

fn biweight_poly(u: &Poly) -> Poly {
    let mut inside = poly_mul(u, u);
    for coefficient in inside.iter_mut() {
        *coefficient = -*coefficient;
    }
    inside[0] += 1.0;
    poly_mul(&inside, &inside)
}

fn envelope_poly(params: &WindShellParams, h0: f32, h1: f32, h_mid: f32) -> Poly {
    let mut envelope = [0.0f32; POLY_TERMS];
    let fade_start = params.fade_start();
    if h_mid <= fade_start {
        envelope[0] = 1.0;
        return envelope;
    }
    let v0 = (h0 - fade_start) / params.top_fade;
    let v1 = h1 / params.top_fade;
    envelope[0] = 1.0 - 3.0 * v0 * v0 + 2.0 * v0 * v0 * v0;
    envelope[1] = -6.0 * v0 * v1 + 6.0 * v0 * v0 * v1;
    envelope[2] = -3.0 * v1 * v1 + 6.0 * v0 * v1 * v1;
    envelope[3] = 2.0 * v1 * v1 * v1;
    envelope
}

fn poly_moments(poly: &Poly) -> f32 {
    poly.iter()
        .enumerate()
        .map(|(n, coefficient)| coefficient / (n as f32 + 1.0))
        .sum()
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

fn sample_point(start: Vector3<f32>, direction: Vector3<f32>, distance: f32) -> [f32; 3] {
    let point = start + direction * distance;
    [point.x, point.y, point.z]
}

/// Envelope times wall on the piece as a polynomial in sigma; `None` when the piece holds no shell.
fn shell_piece_poly(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    s0: f32,
    s1: f32,
) -> Option<Poly> {
    let length = s1 - s0;
    if length <= EMPTY_INTERVAL_EPSILON {
        return None;
    }
    let start = origin + direction * s0;
    let inv_height = 1.0 / params.height;
    let h0 = start.y * inv_height;
    let h1 = length * direction.y * inv_height;
    let h_mid = h0 + 0.5 * h1;
    if !(0.0..=params.h_top).contains(&h_mid) {
        return None;
    }

    let q0 = start.x * start.x + start.z * start.z;
    let q1 = 2.0 * length * (start.x * direction.x + start.z * direction.z);
    let q2 = length * length * (direction.x * direction.x + direction.z * direction.z);

    let radius_0 = params.wall_radius(h0);
    let radius_1 = params.wall_radius_slope * h1;
    let inv_width = 1.0 / params.wall_width_q;
    let u = poly_from_quadratic(
        (q0 - radius_0 * radius_0 - params.spread_offset) * inv_width,
        (q1 - 2.0 * radius_0 * radius_1) * inv_width,
        (q2 - radius_1 * radius_1) * inv_width,
    );
    let u_mid = u[0] + 0.5 * u[1] + 0.25 * u[2];
    if u_mid.abs() >= 1.0 {
        return None;
    }

    let wall = biweight_poly(&u);
    let inv_h_top = 1.0 / params.h_top;
    let envelope = envelope_poly(params, h0 * inv_h_top, h1 * inv_h_top, h_mid * inv_h_top);
    let mut density = poly_mul(&envelope, &wall);
    for coefficient in density.iter_mut() {
        *coefficient *= params.wall_strength;
    }
    Some(density)
}

/// Optical depth of the wall and envelope alone on the piece [s0, s1] (shadow rays).
pub fn wind_shadow_piece_optical_depth(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    s0: f32,
    s1: f32,
) -> f32 {
    let Some(density) = shell_piece_poly(params, origin, direction, s0, s1) else {
        return 0.0;
    };
    ((s1 - s0) * params.sigma_t * poly_moments(&density)).max(0.0)
}

/// Exact optical depth of the streaked and eddy-modulated shell on the ray piece [s0, s1],
/// which must not cross a knot. Puffs are added by `wind_puff_piece_optical_depth`.
pub fn wind_piece_optical_depth(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    s0: f32,
    s1: f32,
) -> f32 {
    let Some(mut density) = shell_piece_poly(params, origin, direction, s0, s1) else {
        return 0.0;
    };
    let length = s1 - s0;
    let start = origin + direction * s0;

    if params.streak_amplitude > 0.0 {
        let sigma_0 = wind_streak_sigma(params, start);
        let sigma_1 = wind_streak_sigma(params, start + direction * length);

        let mut streak_poly = [0.0f32; POLY_TERMS];
        streak_poly[0] = sigma_0;
        streak_poly[1] = sigma_1 - sigma_0;

        density = poly_mul(&density, &streak_poly);
    }

    if params.eddy_amplitude > 0.0 {
        let mut total = 0.0f32;
        let mut sigma_a = eddy_sigma(params, [start.x, start.y, start.z]);
        for j in 0..EDDY_SPLITS {
            let a = j as f32 / EDDY_SPLITS as f32;
            let b = (j + 1) as f32 / EDDY_SPLITS as f32;
            let sigma_b = eddy_sigma(params, sample_point(start, direction, b * length));
            let slope = (sigma_b - sigma_a) / (b - a);
            let intercept = sigma_a - slope * a;
            let mut pow_a = 1.0f32;
            let mut pow_b = 1.0f32;
            for n in 0..POLY_TERMS {
                let m1 = (pow_b * b * b - pow_a * a * a) / (n + 2) as f32;
                let m0 = (pow_b * b - pow_a * a) / (n + 1) as f32;
                total += density[n] * (intercept * m0 + slope * m1);
                pow_a *= a;
                pow_b *= b;
            }
            sigma_a = sigma_b;
        }
        return (length * params.sigma_t * total).max(0.0);
    }

    (length * params.sigma_t * poly_moments(&density)).max(0.0)
}

// Integral of (1 - u^2)^2 over sigma in [0, 1] for u = u0 + u1 sigma + u2 sigma^2.
fn biweight_quadratic_integral(u0: f32, u1: f32, u2: f32) -> f32 {
    let u_squared = [
        u0 * u0,
        2.0 * u0 * u1,
        u1 * u1 + 2.0 * u0 * u2,
        2.0 * u1 * u2,
        u2 * u2,
    ];
    let mut second_moment = 0.0f32;
    let mut fourth_moment = 0.0f32;
    for (i, &ci) in u_squared.iter().enumerate() {
        second_moment += ci / (i as f32 + 1.0);
        for (j, &cj) in u_squared.iter().enumerate() {
            fourth_moment += ci * cj / ((i + j) as f32 + 1.0);
        }
    }
    1.0 - 2.0 * second_moment + fourth_moment
}

/// Optical depth of the puffs whose entry/exit knots enclose the piece [s0, s1].
pub fn wind_puff_piece_optical_depth(
    params: &WindShellParams,
    puffs: &WindRayPuffs,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    s0: f32,
    s1: f32,
) -> f32 {
    let length = s1 - s0;
    if length <= EMPTY_INTERVAL_EPSILON || puffs.count == 0 {
        return 0.0;
    }
    let s_mid = 0.5 * (s0 + s1);
    let start = origin + direction * s0;
    let dd = direction.dot(direction);

    let mut total = 0.0f32;
    for k in 0..puffs.count {
        if s_mid <= puffs.enter[k] || s_mid >= puffs.exit[k] {
            continue;
        }
        let puff = &params.puffs[puffs.index[k]];
        let dx = start - Vector3::new(puff[0], puff[1], puff[2]);
        let inv_r_sq = 1.0 / (puff[3] * puff[3]);
        let u0 = dx.dot(dx) * inv_r_sq;
        let u1 = 2.0 * length * dx.dot(direction) * inv_r_sq;
        let u2 = length * length * dd * inv_r_sq;
        total += biweight_quadratic_integral(u0, u1, u2);
    }
    (length * params.sigma_t * params.puff_strength * params.wall_strength * total).max(0.0)
}

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
    let (knots, count, puffs) = wind_ray_knots(params, origin, direction, t_near, t_far);
    let mut total = 0.0f32;
    for i in 1..count {
        total += wind_piece_optical_depth(params, origin, direction, knots[i - 1], knots[i])
            + wind_puff_piece_optical_depth(
                params,
                &puffs,
                origin,
                direction,
                knots[i - 1],
                knots[i],
            );
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
    if t_far <= t_near {
        return 0.0;
    }
    let (knots, count) = wind_shell_knots(params, origin, direction, t_near, t_far);
    let mut total = 0.0f32;
    for i in 1..count {
        total += wind_shadow_piece_optical_depth(params, origin, direction, knots[i - 1], knots[i]);
    }
    total
}

/// Shadow optical depth from `origin` toward `direction` up to the cone boundary.
pub fn wind_optical_depth_toward(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
) -> f32 {
    let mut t_near = 0.0f32;
    let mut t_far = SHADOW_RAY_T_MAX;
    if !clamp_ray_to_wind_cone(params, origin, direction, &mut t_near, &mut t_far) {
        return 0.0;
    }
    wind_shadow_optical_depth(params, origin, direction, t_near.max(0.0), t_far)
}
