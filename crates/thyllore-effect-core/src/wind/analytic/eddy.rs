use crate::wind::analytic::motion::rotation_phase;
use crate::wind::analytic::shell_integral::WindShellParams;
use cgmath::{Matrix3, Vector3};
use thyllore_math_core::{gradient_noise3, smoothstep, Mat3};

const OCTAVE_ROTATION: Mat3 = Matrix3::new(
    0.784750, 0.509329, -0.353201, -0.045714, 0.615862, 0.786527, 0.618124, -0.601081, 0.506581,
);

fn rotate(p: [f32; 3]) -> [f32; 3] {
    let rotated = OCTAVE_ROTATION * Vector3::new(p[0], p[1], p[2]);
    [rotated.x, rotated.y, rotated.z]
}

fn rotate_and_double(p: [f32; 3]) -> [f32; 3] {
    rotate(p).map(|component| 2.0 * component)
}

// Difference against the antipode (theta + pi) has an exact zero mean around every ring,
// so no height can become a uniformly dense or empty band.
fn antipodal_octave(p: [f32; 3], antipode: [f32; 3]) -> f32 {
    (gradient_noise3(p) - gradient_noise3(antipode)) * std::f32::consts::FRAC_1_SQRT_2
}

pub const EDDY_OCTAVE_COUNT: usize = 3;
// Lattice distance between consecutive cell nodes at which an octave starts to fade and is gone (Nyquist = 0.5).
pub(crate) const EDDY_FADE_START: f32 = 0.25;
pub(crate) const EDDY_FADE_END: f32 = 0.5;

pub type EddyOctaveRings = [[[f32; 3]; 2]; EDDY_OCTAVE_COUNT];

fn eddy_octave_weight(point: [f32; 3], point_ahead: [f32; 3], octave: usize) -> f32 {
    let distance = ((point[0] - point_ahead[0]).powi(2)
        + (point[1] - point_ahead[1]).powi(2)
        + (point[2] - point_ahead[2]).powi(2))
    .sqrt();
    let lattice_step = 2f32.powi(octave as i32) * distance;
    1.0 - smoothstep(EDDY_FADE_START, EDDY_FADE_END, lattice_step)
}

/// Octaves whose lattice step over one cell exceeds the fade band are dropped; the rest are rescaled
/// to keep the variance, so the eroded mean does not drift with the cell length.
pub fn eddy_noise_fbm(rings: &EddyOctaveRings, rings_ahead: &EddyOctaveRings) -> f32 {
    let mut sum = 0.0;
    let mut amplitude = 0.5f32;
    let mut full_variance = 0.0f32;
    let mut kept_variance = 0.0f32;
    for (octave, (ring, ring_ahead)) in rings.iter().zip(rings_ahead).enumerate() {
        full_variance += amplitude * amplitude;
        let weight = eddy_octave_weight(ring[0], ring_ahead[0], octave);
        if weight > 0.0 {
            let mut p = rotate(ring[0]);
            let mut antipode = rotate(ring[1]);
            for _ in 0..octave {
                p = rotate_and_double(p);
                antipode = rotate_and_double(antipode);
            }
            sum += weight * amplitude * antipodal_octave(p, antipode);
            kept_variance += (weight * amplitude).powi(2);
        }
        amplitude *= 0.5;
    }
    if kept_variance > 0.0 {
        sum *= (full_variance / kept_variance).sqrt();
    }
    (0.5 + sum * (1.0 / 0.875)).clamp(0.0, 1.0)
}

pub struct EddyGeometry {
    pub theta: f32,
    pub height: f32,
    pub radius: f32,
    pub radial_coord: f32,
    pub ring_radius: f32,
}

const EDDY_MIN_RADIUS_SQ: f32 = 1e-4;

pub fn eddy_geometry(params: &WindShellParams, local: [f32; 3]) -> EddyGeometry {
    let r = (local[0] * local[0] + local[2] * local[2]).sqrt();
    let theta = local[2].atan2(local[0]);
    let h = local[1];
    let wall_radius = params.wall_radius(h);

    EddyGeometry {
        theta,
        height: h,
        radius: r,
        radial_coord: (r - wall_radius) / params.eddy_cell_radial,
        ring_radius: wall_radius / params.eddy_cell_theta,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EddyReseedLayer {
    A,
    B,
}

impl EddyReseedLayer {
    fn speed_offset(self) -> f32 {
        match self {
            EddyReseedLayer::A => 0.25,
            EddyReseedLayer::B => -0.25,
        }
    }
}

pub fn eddy_layer_coords(
    params: &WindShellParams,
    geometry: &EddyGeometry,
    age: f32,
    seed: f32,
    layer: EddyReseedLayer,
) -> EddyOctaveRings {
    let wall_radius = params.wall_radius(geometry.height);
    let phase = rotation_phase(
        params.time,
        params.circulation,
        wall_radius * wall_radius,
        params.spread_start,
        params.spread_rate,
    );
    let shear = (params.wall_radius_sq(geometry.height)
        / (geometry.radius * geometry.radius).max(EDDY_MIN_RADIUS_SQ))
    .powf(params.eddy_shear);

    let rho = geometry.ring_radius + geometry.radial_coord;
    let u_h = (geometry.height - params.eddy_rise_speed * age) / params.eddy_cell_height;

    std::array::from_fn(|octave| {
        let spread_coefficient = (octave as f32 - 1.0) * 0.5 + layer.speed_offset();
        let speed_factor = 1.0 + params.eddy_speed_spread * spread_coefficient;
        let sheared_theta = geometry.theta - phase * shear * speed_factor;
        let ring_x = rho * sheared_theta.cos();
        let ring_y = rho * sheared_theta.sin();

        [
            [ring_x + seed, ring_y + seed * 0.37, u_h + seed * 0.61],
            [-ring_x + seed, -ring_y + seed * 0.37, u_h + seed * 0.61],
        ]
    })
}

/// `step_ahead` is the ray step to the next cell node; zero gives the pointwise field with every octave kept.
pub fn eddy_sigma(params: &WindShellParams, local: [f32; 3], step_ahead: [f32; 3]) -> f32 {
    let reseed_period = params.eddy_reseed_period;
    let t = params.time;

    let k_a = (t / reseed_period).floor();
    let age_a = t - k_a * reseed_period;
    let w_a = 1.0 - (2.0 * age_a / reseed_period - 1.0).abs();

    let k_b = (t / reseed_period + 0.5).floor();
    let age_b = t + 0.5 * reseed_period - k_b * reseed_period;
    let w_b = 1.0 - w_a;

    let geometry = eddy_geometry(params, local);
    let geometry_ahead = eddy_geometry(
        params,
        [
            local[0] + step_ahead[0],
            local[1] + step_ahead[1],
            local[2] + step_ahead[2],
        ],
    );
    let seed_a = 17.0 * k_a + 3.0;
    let seed_b = 17.0 * k_b + 3.0;

    let rings_a = eddy_layer_coords(params, &geometry, age_a, seed_a, EddyReseedLayer::A);
    let rings_b = eddy_layer_coords(params, &geometry, age_b, seed_b, EddyReseedLayer::B);
    let rings_a_ahead =
        eddy_layer_coords(params, &geometry_ahead, age_a, seed_a, EddyReseedLayer::A);
    let rings_b_ahead =
        eddy_layer_coords(params, &geometry_ahead, age_b, seed_b, EddyReseedLayer::B);
    let noise_a = eddy_noise_fbm(&rings_a, &rings_a_ahead);
    let noise_b = eddy_noise_fbm(&rings_b, &rings_b_ahead);

    let noise = w_a * noise_a + w_b * noise_b;
    let eroded = ((noise - params.eddy_erosion) / (1.0 - params.eddy_erosion)).clamp(0.0, 1.0);
    1.0 + params.eddy_amplitude * (2.0 * eroded - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wind::WindTornadoEffect;

    fn shared_embedding_fbm(p: [f32; 3], antipode: [f32; 3]) -> f32 {
        let p0 = rotate(p);
        let q0 = rotate(antipode);
        let p1 = rotate_and_double(p0);
        let q1 = rotate_and_double(q0);
        let p2 = rotate_and_double(p1);
        let q2 = rotate_and_double(q1);
        let sum = 0.5 * antipodal_octave(p0, q0)
            + 0.25 * antipodal_octave(p1, q1)
            + 0.125 * antipodal_octave(p2, q2);
        (0.5 + sum * (1.0 / 0.875)).clamp(0.0, 1.0)
    }

    fn params_with_speed_spread(eddy_speed_spread: f32) -> WindShellParams {
        WindShellParams::from_effect(&WindTornadoEffect {
            time: 1.3,
            circulation: 6.0,
            spread_start: 0.25,
            spread_rate: 0.08,
            eddy_shear: 0.6,
            eddy_speed_spread,
            ..WindTornadoEffect::default()
        })
    }

    const SAMPLE_LOCAL: [f32; 3] = [0.7, 0.9, -0.4];
    const SAMPLE_AGE: f32 = 0.42;
    const SAMPLE_SEED: f32 = 20.0;

    #[test]
    fn zero_speed_spread_gives_every_octave_the_same_ring() {
        let params = params_with_speed_spread(0.0);
        let geometry = eddy_geometry(&params, SAMPLE_LOCAL);

        for layer in [EddyReseedLayer::A, EddyReseedLayer::B] {
            let rings = eddy_layer_coords(&params, &geometry, SAMPLE_AGE, SAMPLE_SEED, layer);
            assert_eq!(rings[1], rings[0], "octave 1 ring differs for {layer:?}");
            assert_eq!(rings[2], rings[0], "octave 2 ring differs for {layer:?}");
        }
    }

    #[test]
    fn zero_speed_spread_matches_the_shared_embedding() {
        let params = params_with_speed_spread(0.0);
        let geometry = eddy_geometry(&params, SAMPLE_LOCAL);
        let rings = eddy_layer_coords(
            &params,
            &geometry,
            SAMPLE_AGE,
            SAMPLE_SEED,
            EddyReseedLayer::A,
        );

        assert_eq!(
            eddy_noise_fbm(&rings, &rings),
            shared_embedding_fbm(rings[0][0], rings[0][1])
        );
    }

    #[test]
    fn speed_spread_separates_the_octave_rings() {
        let params = params_with_speed_spread(0.5);
        let geometry = eddy_geometry(&params, SAMPLE_LOCAL);
        let rings = eddy_layer_coords(
            &params,
            &geometry,
            SAMPLE_AGE,
            SAMPLE_SEED,
            EddyReseedLayer::A,
        );

        assert_ne!(rings[1], rings[0]);
        assert_ne!(rings[2], rings[0]);
    }
}
