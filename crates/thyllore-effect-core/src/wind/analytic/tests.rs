use super::*;
use crate::wind::analytic::eddy::EDDY_OCTAVE_COUNT;
use crate::wind::analytic::motion::rotation_phase;
use crate::wind::analytic::shell_integral::{ACTIVE_CELLS_MIN, MODULATION_CELLS, POLY_TERMS};
use crate::wind::WindTornadoEffect;
use crate::wind::{
    WIND_SHADOW_VOLUME_HEIGHT, WIND_SHADOW_VOLUME_RADIAL, WIND_SHADOW_VOLUME_SLOTS,
    WIND_SHADOW_VOLUME_THETA,
};
use cgmath::{InnerSpace, Vector3};
use std::f32::consts::PI;

const REFERENCE_STEPS: usize = 20000;

fn params() -> WindShellParams {
    WindShellParams::from_effect(&WindTornadoEffect::default())
}

fn midpoint_optical_depth(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> f64 {
    let step = (t_far - t_near) as f64 / REFERENCE_STEPS as f64;
    let mut total = 0.0f64;
    for i in 0..REFERENCE_STEPS {
        let t = t_near as f64 + (i as f64 + 0.5) * step;
        let point = origin + direction * t as f32;
        total += wind_density_at(params, point) as f64 * step;
    }
    total
}

fn evolving_params() -> WindShellParams {
    let effect = WindTornadoEffect {
        time: 1.0,
        rise_initial_height: 0.3,
        rise_duration: 2.0,
        spread_start: 0.25,
        spread_rate: 0.08,
        dissipate_start: 0.5,
        dissipate_time: 1.5,
        ..WindTornadoEffect::default()
    };
    WindShellParams::from_effect(&effect)
}

fn assert_closed_form_matches_reference(origin: Vector3<f32>, direction: Vector3<f32>) {
    assert_closed_form_matches_reference_for(params(), origin, direction);
}

fn assert_closed_form_matches_reference_for(
    params: WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
) {
    let direction = direction.normalize();
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    assert!(
        clamp_ray_to_wind_cone(&params, origin, direction, &mut t_near, &mut t_far),
        "ray must hit the envelope"
    );
    let closed = wind_optical_depth(&params, origin, direction, t_near, t_far) as f64;
    let reference = midpoint_optical_depth(&params, origin, direction, t_near, t_far);
    assert!(
        reference > 1e-3,
        "reference {reference} too small to compare"
    );
    let relative = (closed - reference).abs() / reference;
    assert!(
        relative < 2e-3,
        "closed {closed} vs reference {reference} (rel {relative}) for o={origin:?} d={direction:?}"
    );
}

const EDDY_RING_SAMPLES: usize = 256;

fn storm_effect_at(time: f32) -> WindTornadoEffect {
    WindTornadoEffect {
        time,
        circulation: 2.0,
        wall_radius_base: 0.35,
        wall_radius_top: 0.6,
        eddy_amplitude: 1.0,
        eddy_shear: 1.0,
        eddy_speed_spread: 0.0,
        spread_start: 0.0,
        spread_rate: 0.0,
        ..WindTornadoEffect::default()
    }
}

fn sample_eddy_ring(params: &WindShellParams, height: f32) -> Vec<f32> {
    let radius = params.wall_radius(height);
    (0..EDDY_RING_SAMPLES)
        .map(|index| {
            let theta = 2.0 * PI * index as f32 / EDDY_RING_SAMPLES as f32;
            eddy_sigma(params, [radius * theta.cos(), height, radius * theta.sin()])
        })
        .collect()
}

fn correlate_cyclic_shift(before: &[f32], after: &[f32], shift: usize) -> f32 {
    let count = before.len();
    let before_mean = before.iter().sum::<f32>() / count as f32;
    let after_mean = after.iter().sum::<f32>() / count as f32;
    (0..count)
        .map(|index| (before[index] - before_mean) * (after[(index + shift) % count] - after_mean))
        .sum()
}

fn estimate_rotation_angle(before: &[f32], after: &[f32]) -> f32 {
    let count = before.len();
    let correlations: Vec<f32> = (0..count)
        .map(|shift| correlate_cyclic_shift(before, after, shift))
        .collect();

    let (best_shift, _) = correlations.iter().enumerate().fold(
        (0usize, f32::NEG_INFINITY),
        |(best_index, best_value), (index, &value)| {
            if value > best_value {
                (index, value)
            } else {
                (best_index, best_value)
            }
        },
    );

    let previous = correlations[(best_shift + count - 1) % count];
    let next = correlations[(best_shift + 1) % count];
    let curvature = previous - 2.0 * correlations[best_shift] + next;
    let sub_sample = if curvature < 0.0 {
        0.5 * (previous - next) / curvature
    } else {
        0.0
    };

    (best_shift as f32 + sub_sample) * 2.0 * PI / count as f32
}

#[test]
fn eddy_rotation_matches_rankine_angular_velocity() {
    let time = 0.3;
    let delta_time = 0.05;
    let heights = [0.1f32, 0.9];

    let before = storm_effect_at(time);
    let after = storm_effect_at(time + delta_time);
    let params_before = WindShellParams::from_effect(&before);
    let params_after = WindShellParams::from_effect(&after);

    let sample_step = 2.0 * PI / EDDY_RING_SAMPLES as f32;
    let mut measured_angles = Vec::new();

    for &height in &heights {
        let measured = estimate_rotation_angle(
            &sample_eddy_ring(&params_before, height),
            &sample_eddy_ring(&params_after, height),
        );

        let radius_sq = params_before.wall_radius(height).powi(2);
        let expected = rotation_phase(
            time + delta_time,
            before.circulation,
            radius_sq,
            before.spread_start,
            before.spread_rate,
        ) - rotation_phase(
            time,
            before.circulation,
            radius_sq,
            before.spread_start,
            before.spread_rate,
        );

        assert!(
            (measured - expected).abs() <= 2.0 * sample_step,
            "height {height}: measured {measured} vs expected {expected} (step {sample_step})"
        );
        measured_angles.push(measured);
    }

    let expected_ratio = params_before.wall_radius(heights[1]).powi(2)
        / params_before.wall_radius(heights[0]).powi(2);
    let measured_ratio = measured_angles[0] / measured_angles[1];
    assert!(
        (measured_ratio - expected_ratio).abs() <= 0.1 * expected_ratio,
        "rotation ratio {measured_ratio} vs expected {expected_ratio}"
    );
}

#[test]
fn horizontal_ray_through_the_wall_matches_the_midpoint_reference() {
    assert_closed_form_matches_reference(
        Vector3::new(-5.0, 0.8, 0.05),
        Vector3::new(1.0, 0.0, 0.0),
    );
}

#[test]
fn oblique_ray_crossing_the_top_fade_matches_the_midpoint_reference() {
    assert_closed_form_matches_reference(
        Vector3::new(-3.0, 0.2, 0.3),
        Vector3::new(1.0, 0.55, -0.1),
    );
}

#[test]
fn ray_through_the_axis_matches_the_midpoint_reference() {
    assert_closed_form_matches_reference(
        Vector3::new(-4.0, 1.0, 0.0),
        Vector3::new(1.0, 0.02, 0.0),
    );
}

#[test]
fn ray_starting_inside_the_wall_matches_the_midpoint_reference() {
    assert_closed_form_matches_reference(Vector3::new(0.4, 0.5, 0.0), Vector3::new(0.3, 0.2, 1.0));
}

#[test]
fn near_axial_ray_matches_the_midpoint_reference() {
    assert_closed_form_matches_reference(
        Vector3::new(0.42, -1.0, 0.0),
        Vector3::new(0.001, 1.0, 0.0),
    );
}

#[test]
fn rising_and_spreading_wall_matches_the_midpoint_reference() {
    assert_closed_form_matches_reference_for(
        evolving_params(),
        Vector3::new(-3.0, 0.2, 0.3),
        Vector3::new(1.0, 0.35, -0.1),
    );
    assert_closed_form_matches_reference_for(
        evolving_params(),
        Vector3::new(-5.0, 0.5, 0.05),
        Vector3::new(1.0, 0.0, 0.0),
    );
}

fn streaked_params() -> WindShellParams {
    let effect = WindTornadoEffect {
        time: 1.0,
        circulation: 2.0,
        streak_order: 3.0,
        streak_twist: 4.0,
        streak_amplitude: 0.5,
        ..WindTornadoEffect::default()
    };
    WindShellParams::from_effect(&effect)
}

fn streaked_midpoint_optical_depth(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> f64 {
    let step = (t_far - t_near) as f64 / REFERENCE_STEPS as f64;
    let mut total = 0.0f64;
    for i in 0..REFERENCE_STEPS {
        let t = t_near as f64 + (i as f64 + 0.5) * step;
        let point = origin + direction * t as f32;
        total += (wind_density_at(params, point) * wind_streak_sigma(params, point)) as f64 * step;
    }
    total
}

#[test]
fn streaked_wall_matches_the_midpoint_reference() {
    let params = streaked_params();
    let origin = Vector3::new(-5.0, 0.5, 0.25);
    let direction = Vector3::new(1.0, 0.1, 0.0).normalize();
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    assert!(
        clamp_ray_to_wind_cone(&params, origin, direction, &mut t_near, &mut t_far),
        "ray must hit the envelope"
    );
    let closed = wind_optical_depth(&params, origin, direction, t_near, t_far) as f64;
    let reference = streaked_midpoint_optical_depth(&params, origin, direction, t_near, t_far);
    assert!(
        reference > 1e-3,
        "reference {reference} too small to compare"
    );
    let relative = (closed - reference).abs() / reference;
    assert!(
        relative < 5e-2,
        "closed {closed} vs reference {reference} (rel {relative})"
    );
}

#[test]
fn wall_evolution_moves_the_top_and_dims_the_wall() {
    let evolving = evolving_params();
    let still = params();
    assert!(evolving.h_top < still.h_top);
    assert!(evolving.spread_offset > 0.0);
    assert!(evolving.wall_strength < still.wall_strength);
    assert_eq!(
        wind_density_at(
            &evolving,
            Vector3::new(0.35, evolving.h_top * still.height + 0.1, 0.0)
        ),
        0.0
    );
}

#[test]
fn ray_missing_the_envelope_has_no_optical_depth() {
    let params = params();
    let origin = Vector3::new(-5.0, 0.5, 5.0);
    let direction = Vector3::new(1.0, 0.0, 0.0);
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    assert!(!clamp_ray_to_wind_cone(
        &params,
        origin,
        direction,
        &mut t_near,
        &mut t_far
    ));
}

#[test]
fn density_vanishes_outside_the_height_slab_and_the_shell_supports() {
    let params = params();
    assert_eq!(wind_density_at(&params, Vector3::new(0.35, -0.1, 0.0)), 0.0);
    assert_eq!(wind_density_at(&params, Vector3::new(0.35, 2.1, 0.0)), 0.0);
    assert_eq!(wind_density_at(&params, Vector3::new(3.0, 1.0, 0.0)), 0.0);
    assert!(wind_density_at(&params, Vector3::new(0.35, 0.5, 0.0)) > 0.0);
    assert_eq!(wind_density_at(&params, Vector3::new(0.0, 0.5, 0.0)), 0.0);
}

#[test]
fn knots_are_sorted_and_bracketed() {
    let params = params();
    let origin = Vector3::new(-3.0, 0.2, 0.3);
    let direction = Vector3::new(1.0, 0.55, -0.1).normalize();
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    clamp_ray_to_wind_cone(&params, origin, direction, &mut t_near, &mut t_far);
    let (knots, count, _puffs) = wind_ray_knots(&params, origin, direction, t_near, t_far);
    assert!(count >= 2);
    assert_eq!(knots[0], t_near);
    assert_eq!(knots[count - 1], t_far);
    for i in 1..count {
        assert!(knots[i - 1] <= knots[i], "knots {:?}", &knots[..count]);
    }
}

fn total_mass(params: &WindShellParams, q_max: f32) -> f64 {
    let n_h = 64;
    let n_q = 512;
    let h_top = params.h_top * params.height;
    let dh = h_top / n_h as f32;
    let dq = q_max / n_q as f32;
    let mut mass: f64 = 0.0;
    for j in 0..n_h {
        let h = (j as f32 + 0.5) * dh;
        for i in 0..n_q {
            let q = (i as f32 + 0.5) * dq;
            let r = q.sqrt();
            let rho = wind_density_at(params, Vector3::new(r, h, 0.0));
            mass += rho as f64;
        }
    }
    mass * (dq * dh) as f64 * std::f64::consts::PI
}

fn envelope_q_max(params: &WindShellParams) -> f32 {
    let wall_top_r = params.wall_radius_base + params.wall_radius_slope;
    let wall_max_r =
        (wall_top_r * wall_top_r + params.spread_offset).sqrt() + params.wall_width_q.sqrt();
    (wall_max_r + 0.1) * (wall_max_r + 0.1)
}

#[test]
fn mass_is_conserved_under_wall_spread() {
    let effect = WindTornadoEffect {
        time: 0.0,
        rise_initial_height: 1.0,
        rise_duration: 0.0,
        spread_start: 0.5,
        spread_rate: 0.1,
        dissipate_start: 0.0,
        dissipate_time: 0.0,
        ..WindTornadoEffect::default()
    };

    let times = [0.3, 1.5, 5.0];

    let q_max = {
        let mut e = effect.clone();
        e.time = *times.last().unwrap();
        let p = WindShellParams::from_effect(&e);
        envelope_q_max(&p)
    };

    let masses: Vec<f64> = times
        .iter()
        .map(|&t| {
            let mut e = effect.clone();
            e.time = t;
            let p = WindShellParams::from_effect(&e);
            total_mass(&p, q_max)
        })
        .collect();

    assert!(masses[0] > 1e-3, "reference mass too small: {:?}", masses);

    for i in 1..times.len() {
        let rel_err = (masses[i] - masses[0]).abs() / masses[0];
        assert!(
            rel_err < 1e-3,
            "wall spread mass not conserved: t[0]={} mass={:.6}, t[{}]={} mass={:.6}, rel_err={:.6}",
            times[0], masses[0], i, times[i], masses[i], rel_err
        );
    }
}

#[test]
fn eddy_sigma_is_continuous_across_the_theta_seam() {
    let params = eddy_params_with_amplitude(1.0);
    let epsilon = 1e-4f32;
    for j in 0..16 {
        let h = params.h_top * (j as f32 + 0.5) / 16.0;
        let radius = params.wall_radius(h);
        let before = [
            radius * (PI - epsilon).cos(),
            h,
            radius * (PI - epsilon).sin(),
        ];
        let after = [
            radius * (-PI + epsilon).cos(),
            h,
            radius * (-PI + epsilon).sin(),
        ];
        let gap = (eddy_sigma(&params, before) - eddy_sigma(&params, after)).abs();
        assert!(
            gap < 1e-2,
            "eddy sigma jumps by {gap} across theta = pi at h = {h}"
        );
    }
}

#[test]
fn eddy_noise_stays_in_the_unit_interval_and_is_not_height_banded() {
    let params = eddy_params_with_amplitude(1.0);
    let theta_samples = 48;
    let height_samples = 48;
    let mut row_means = Vec::with_capacity(height_samples);
    for j in 0..height_samples {
        let h = params.h_top * j as f32 / (height_samples - 1) as f32;
        let radius = params.wall_radius(h);
        let mut row = 0.0f32;
        for i in 0..theta_samples {
            let theta = 2.0 * PI * i as f32 / theta_samples as f32;
            let sigma = eddy_sigma(&params, [radius * theta.cos(), h, radius * theta.sin()]);
            assert!((0.0..=2.0).contains(&sigma), "sigma {sigma} outside [0, 2]");
            row += sigma;
        }
        row_means.push(row / theta_samples as f32);
    }
    let spread = row_means.iter().cloned().fold(f32::MIN, f32::max)
        - row_means.iter().cloned().fold(f32::MAX, f32::min);
    assert!(
        spread < 0.25,
        "per-height mean sigma varies by {spread}: the noise is banded along height"
    );
}

fn eddy_params_with_amplitude(amplitude: f32) -> WindShellParams {
    let effect = WindTornadoEffect {
        time: 1.0,
        circulation: 2.0,
        streak_order: 3.0,
        streak_twist: 4.0,
        streak_amplitude: 0.25,
        eddy_amplitude: amplitude,
        eddy_shear: 1.0,
        ..WindTornadoEffect::default()
    };
    WindShellParams::from_effect(&effect)
}

#[test]
fn eddy_sigma_averages_to_one_over_the_wall() {
    let params = eddy_params_with_amplitude(0.5);
    let theta_samples = 64;
    let height_samples = 64;
    let mut total = 0.0f64;
    for i in 0..theta_samples {
        for j in 0..height_samples {
            let theta = 2.0 * 3.14159265 * i as f32 / theta_samples as f32;
            let h = params.h_top * j as f32 / (height_samples - 1) as f32;
            let radius = params.wall_radius(h);
            total += eddy_sigma(&params, [radius * theta.cos(), h, radius * theta.sin()]) as f64;
        }
    }
    let mean = total / (theta_samples * height_samples) as f64;
    assert!(
        (mean - 1.0).abs() < 0.03,
        "eddy sigma mean {mean} must stay near 1 so the eddies do not add mass"
    );
}

fn eddy_midpoint_optical_depth(
    params: &WindShellParams,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> f64 {
    const EDDY_REFERENCE_STEPS: usize = 2048;
    let step = (t_far - t_near) as f64 / EDDY_REFERENCE_STEPS as f64;
    let mut total = 0.0f64;
    for i in 0..EDDY_REFERENCE_STEPS {
        let t = t_near as f64 + (i as f64 + 0.5) * step;
        let point = origin + direction * t as f32;
        let sigma = wind_density_at(params, point)
            * wind_streak_sigma(params, point)
            * eddy_sigma(params, [point.x, point.y, point.z]);
        total += sigma as f64 * step;
    }
    total
}

#[test]
fn eddy_modulated_wall_matches_the_midpoint_reference() {
    let params = eddy_params_with_amplitude(0.5);
    let origin = Vector3::new(-5.0, 0.5, 0.25);
    let direction = Vector3::new(1.0, 0.1, 0.0).normalize();
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    assert!(
        clamp_ray_to_wind_cone(&params, origin, direction, &mut t_near, &mut t_far),
        "ray must hit the envelope"
    );
    let closed = wind_optical_depth(&params, origin, direction, t_near, t_far) as f64;
    let reference = eddy_midpoint_optical_depth(&params, origin, direction, t_near, t_far);
    assert!(
        reference > 1e-3,
        "reference {reference} too small to compare"
    );
    let relative = (closed - reference).abs() / reference;
    assert!(
        relative < 1e-1,
        "closed {closed} vs reference {reference} (rel {relative})"
    );
}

#[test]
fn zero_eddy_amplitude_keeps_the_streak_only_path() {
    let params = eddy_params_with_amplitude(0.0);
    let origin = Vector3::new(-5.0, 0.5, 0.25);
    let direction = Vector3::new(1.0, 0.1, 0.0).normalize();
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    assert!(
        clamp_ray_to_wind_cone(&params, origin, direction, &mut t_near, &mut t_far),
        "ray must hit the envelope"
    );
    let closed = wind_optical_depth(&params, origin, direction, t_near, t_far) as f64;
    let reference = streaked_midpoint_optical_depth(&params, origin, direction, t_near, t_far);
    assert!(
        reference > 1e-3,
        "reference {reference} too small to compare"
    );
    let relative = (closed - reference).abs() / reference;
    assert!(
        relative < 5e-2,
        "closed {closed} vs reference {reference} (rel {relative})"
    );
}

fn eroded_eddy_params(erosion: f32) -> WindShellParams {
    let effect = WindTornadoEffect {
        time: 1.0,
        circulation: 2.0,
        streak_amplitude: 0.25,
        eddy_amplitude: 1.0,
        eddy_shear: 1.0,
        eddy_erosion: erosion,
        ..WindTornadoEffect::default()
    };
    WindShellParams::from_effect(&effect)
}

#[test]
fn eroded_eddy_reaches_zero_density_below_the_noise_floor() {
    let smooth = eroded_eddy_params(0.0);
    let eroded = eroded_eddy_params(0.6);
    let theta_samples = 64;
    let height_samples = 32;
    let mut smooth_min = f32::MAX;
    let mut eroded_zero_count = 0usize;
    for i in 0..theta_samples {
        for j in 0..height_samples {
            let theta = 2.0 * 3.14159265 * i as f32 / theta_samples as f32;
            let h = eroded.h_top * j as f32 / (height_samples - 1) as f32;
            let radius = eroded.wall_radius(h);
            let local = [radius * theta.cos(), h, radius * theta.sin()];
            smooth_min = smooth_min.min(eddy_sigma(&smooth, local));
            if eddy_sigma(&eroded, local) <= 1e-6 {
                eroded_zero_count += 1;
            }
        }
    }
    assert!(
        smooth_min > 0.0,
        "without erosion the eddy sigma must stay positive (min {smooth_min})"
    );
    assert!(
        eroded_zero_count > 0,
        "erosion must carve holes (sigma 0) somewhere on the wall"
    );
}

#[test]
fn eroded_eddy_wall_matches_the_midpoint_reference() {
    let params = eroded_eddy_params(0.4);
    let origin = Vector3::new(-5.0, 0.5, 0.25);
    let direction = Vector3::new(1.0, 0.1, 0.0).normalize();
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    assert!(
        clamp_ray_to_wind_cone(&params, origin, direction, &mut t_near, &mut t_far),
        "ray must hit the envelope"
    );
    let closed = wind_optical_depth(&params, origin, direction, t_near, t_far) as f64;
    let reference = eddy_midpoint_optical_depth(&params, origin, direction, t_near, t_far);
    assert!(
        reference > 1e-3,
        "reference {reference} too small to compare"
    );
    let relative = (closed - reference).abs() / reference;
    assert!(
        relative < 1e-1,
        "closed {closed} vs reference {reference} (rel {relative})"
    );
}

fn puff_effect() -> WindTornadoEffect {
    WindTornadoEffect {
        time: 1.0,
        puff_strength: 1.0,
        puff_count_theta: 4,
        puff_count_height: 2,
        puff_radius: 0.3,
        puff_offset_q: 0.0,
        ..WindTornadoEffect::default()
    }
}

fn puff_params() -> WindShellParams {
    WindShellParams::from_effect(&puff_effect())
}

#[test]
fn puff_ray_crossing_multiple_puffs_matches_midpoint_reference() {
    let params = puff_params();
    assert!(params.puff_count > 0, "puff_count should be > 0");

    let origin = Vector3::new(-5.0, 0.5, 0.25);
    let direction = Vector3::new(1.0, 0.1, 0.0).normalize();
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    assert!(
        clamp_ray_to_wind_cone(&params, origin, direction, &mut t_near, &mut t_far),
        "ray must hit the envelope"
    );

    let step = (t_far - t_near) as f64 / 2048.0;
    let mut reference = 0.0f64;
    for i in 0..2048 {
        let t = t_near as f64 + (i as f64 + 0.5) * step;
        let point = origin + direction * t as f32;
        reference += wind_density_at(&params, point) as f64 * step;
    }

    let closed = wind_optical_depth(&params, origin, direction, t_near, t_far) as f64;
    assert!(
        reference > 1e-3,
        "reference {reference} too small to compare"
    );
    let relative = (closed - reference).abs() / reference;
    assert!(
        relative <= 1e-3,
        "puff closed {closed} vs reference {reference} (rel {relative})"
    );
}

#[test]
fn puff_count_zero_yields_same_results_as_default() {
    let default_params = params();
    let zero_puff_effect = WindTornadoEffect {
        puff_strength: 0.0,
        puff_count_theta: 0,
        puff_count_height: 0,
        puff_radius: 0.0,
        ..WindTornadoEffect::default()
    };
    let zero_puff_params = WindShellParams::from_effect(&zero_puff_effect);
    assert_eq!(zero_puff_params.puff_count, 0);

    let origin = Vector3::new(-5.0, 0.8, 0.05);
    let direction = Vector3::new(1.0, 0.0, 0.0);
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    clamp_ray_to_wind_cone(&default_params, origin, direction, &mut t_near, &mut t_far);

    let default_depth = wind_optical_depth(&default_params, origin, direction, t_near, t_far);
    let zero_puff_depth = wind_optical_depth(&zero_puff_params, origin, direction, t_near, t_far);
    assert!(
        (default_depth - zero_puff_depth).abs() < 1e-6,
        "puff_count=0 should match default: default={default_depth}, zero={zero_puff_depth}"
    );
}

#[test]
fn puff_knot_count_does_not_exceed_wind_max_knots() {
    let params = puff_params();
    let origin = Vector3::new(-5.0, 0.5, 0.25);
    let direction = Vector3::new(1.0, 0.1, 0.0).normalize();
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    clamp_ray_to_wind_cone(&params, origin, direction, &mut t_near, &mut t_far);

    let (_knots, count, _puffs) = wind_ray_knots(&params, origin, direction, t_near, t_far);
    assert!(
        count <= WIND_MAX_KNOTS,
        "knot count {count} exceeds WIND_MAX_KNOTS={}",
        WIND_MAX_KNOTS
    );
}

#[test]
fn large_puff_count_72_matches_midpoint_reference() {
    let effect = WindTornadoEffect {
        time: 1.0,
        puff_strength: 1.0,
        puff_count_theta: 12,
        puff_count_height: 6,
        puff_radius: 0.12,
        puff_offset_q: 0.0,
        ..WindTornadoEffect::default()
    };
    let params = WindShellParams::from_effect(&effect);
    assert_eq!(params.puff_count, 72, "should have 12*6=72 puffs");

    let origin = Vector3::new(-5.0, 0.5, 0.25);
    let direction = Vector3::new(1.0, 0.1, 0.0).normalize();
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    assert!(
        clamp_ray_to_wind_cone(&params, origin, direction, &mut t_near, &mut t_far),
        "ray must hit the envelope"
    );

    let mut intersecting_count = 0usize;
    for i in 0..params.puff_count {
        let puff = &params.puffs[i];
        let cx = puff[0];
        let cy = puff[1];
        let cz = puff[2];
        let r = puff[3];
        if r <= 0.0 {
            continue;
        }
        let dx = origin.x - cx;
        let dy = origin.y - cy;
        let dz = origin.z - cz;
        let a = direction.x * direction.x + direction.y * direction.y + direction.z * direction.z;
        let b = 2.0 * (dx * direction.x + dy * direction.y + dz * direction.z);
        let c = dx * dx + dy * dy + dz * dz - r * r;
        let discriminant = b * b - 4.0 * a * c;
        if discriminant <= 0.0 {
            continue;
        }
        let sqrt_discriminant = discriminant.sqrt();
        let t0 = (-b - sqrt_discriminant) / (2.0 * a);
        let t1 = (-b + sqrt_discriminant) / (2.0 * a);
        let overlap = t0 <= t_far && t1 >= t_near;
        if overlap {
            intersecting_count += 1;
        }
    }
    assert!(
        (1..=8).contains(&intersecting_count),
        "precondition: expected 1-8 intersecting puffs, got {intersecting_count}"
    );

    let step = (t_far - t_near) as f64 / 2048.0;
    let mut reference = 0.0f64;
    for i in 0..2048 {
        let t = t_near as f64 + (i as f64 + 0.5) * step;
        let point = origin + direction * t as f32;
        reference += wind_density_at(&params, point) as f64 * step;
    }

    let closed = wind_optical_depth(&params, origin, direction, t_near, t_far) as f64;
    assert!(
        reference > 1e-3,
        "reference {reference} too small to compare"
    );
    let relative = (closed - reference).abs() / reference;
    assert!(
        relative <= 1e-3,
        "large puff count closed {closed} vs reference {reference} (rel {relative})"
    );
}

#[test]
fn truncated_ray_9_plus_puffs_analytical_leq_midpoint() {
    let effect = WindTornadoEffect {
        time: 1.0,
        puff_strength: 1.0,
        puff_count_theta: 12,
        puff_count_height: 6,
        puff_radius: 0.3,
        puff_offset_q: 0.0,
        ..WindTornadoEffect::default()
    };
    let params = WindShellParams::from_effect(&effect);
    assert_eq!(params.puff_count, 72, "should have 12*6=72 puffs");

    let origin = Vector3::new(-5.0, 0.5, 0.25);
    let direction = Vector3::new(1.0, 0.1, 0.0).normalize();
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    assert!(
        clamp_ray_to_wind_cone(&params, origin, direction, &mut t_near, &mut t_far),
        "ray must hit the envelope"
    );

    let mut intersecting_count = 0usize;
    for i in 0..params.puff_count {
        let puff = &params.puffs[i];
        let cx = puff[0];
        let cy = puff[1];
        let cz = puff[2];
        let r = puff[3];
        if r <= 0.0 {
            continue;
        }
        let dx = origin.x - cx;
        let dy = origin.y - cy;
        let dz = origin.z - cz;
        let a = direction.x * direction.x + direction.y * direction.y + direction.z * direction.z;
        let b = 2.0 * (dx * direction.x + dy * direction.y + dz * direction.z);
        let c = dx * dx + dy * dy + dz * dz - r * r;
        let discriminant = b * b - 4.0 * a * c;
        if discriminant <= 0.0 {
            continue;
        }
        let sqrt_discriminant = discriminant.sqrt();
        let t0 = (-b - sqrt_discriminant) / (2.0 * a);
        let t1 = (-b + sqrt_discriminant) / (2.0 * a);
        let overlap = t0 <= t_far && t1 >= t_near;
        if overlap {
            intersecting_count += 1;
        }
    }
    assert!(
        intersecting_count >= 9,
        "precondition: expected >= 9 intersecting puffs for truncation test, got {intersecting_count}"
    );

    let step = (t_far - t_near) as f64 / 2048.0;
    let mut reference = 0.0f64;
    for i in 0..2048 {
        let t = t_near as f64 + (i as f64 + 0.5) * step;
        let point = origin + direction * t as f32;
        reference += wind_density_at(&params, point) as f64 * step;
    }

    let closed = wind_optical_depth(&params, origin, direction, t_near, t_far) as f64;
    assert!(
        reference > 1e-3,
        "reference {reference} too small to compare"
    );
    assert!(
        closed <= reference * (1.0 + 1e-5),
        "truncated analytical {closed} should be <= midpoint reference {reference}"
    );
}

fn glsl_int_constant(source: &str, name: &str) -> i64 {
    let prefix = format!("const int {name} = ");
    source
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .and_then(|rest| rest.trim_end_matches(';').parse().ok())
        .unwrap_or_else(|| panic!("{name} not declared in the wind GLSL"))
}

fn wind_glsl_source(relative_path: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../shaders/wind/include")
        .join(relative_path);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} readable", path.display()))
}

fn shell_only_params() -> WindShellParams {
    let effect = WindTornadoEffect {
        time: 1.0,
        streak_amplitude: 0.4,
        eddy_amplitude: 1.0,
        puff_strength: 1.0,
        puff_count_theta: 4,
        puff_count_height: 2,
        puff_radius: 0.3,
        puff_offset_q: 0.0,
        ..WindTornadoEffect::default()
    };
    WindShellParams::from_effect(&effect)
}

fn wall_envelope_density(params: &WindShellParams, local: Vector3<f32>) -> f32 {
    let h = local.y / params.height;
    let q = local.x * local.x + local.z * local.z;
    let u = (q - params.wall_radius_sq(h)) / params.wall_width_q;
    let inside = (1.0 - u * u).max(0.0);
    params.sigma_t * wind_envelope_height(params, h) * params.wall_strength * inside * inside
}

#[test]
fn shadow_optical_depth_keeps_only_the_wall_and_envelope() {
    let params = shell_only_params();
    let origin = Vector3::new(-5.0, 0.5, 0.25);
    let direction = Vector3::new(1.0, 0.1, 0.0).normalize();
    let mut t_near = 0.0;
    let mut t_far = 1e4;
    assert!(clamp_ray_to_wind_cone(
        &params,
        origin,
        direction,
        &mut t_near,
        &mut t_far
    ));

    let step = (t_far - t_near) as f64 / 4096.0;
    let mut reference = 0.0f64;
    for i in 0..4096 {
        let t = t_near as f64 + (i as f64 + 0.5) * step;
        reference += wall_envelope_density(&params, origin + direction * t as f32) as f64 * step;
    }

    let closed = wind_shadow_optical_depth(&params, origin, direction, t_near, t_far) as f64;
    assert!(reference > 1e-3, "reference {reference} too small");
    let relative = (closed - reference).abs() / reference;
    assert!(
        relative <= 1e-3,
        "shadow closed {closed} vs wall+envelope reference {reference} (rel {relative})"
    );
    let modulated = wind_optical_depth(&params, origin, direction, t_near, t_far) as f64;
    assert!(
        (modulated - closed).abs() > 1e-3,
        "the modulated depth {modulated} should differ from the shadow depth {closed}"
    );
}

#[test]
fn shadow_radial_extent_covers_the_shell_and_every_puff() {
    let params = puff_params();
    let extent = wind_shadow_radial_extent(&params);

    let base_radius = params.wall_radius_sq(0.0).sqrt();
    let shell_half_width = (params.wall_radius_sq(0.0) + params.wall_width_q).sqrt() - base_radius;
    assert!(
        extent > shell_half_width,
        "extent {extent} must cover the shell {shell_half_width}"
    );

    for puff in &params.puffs[..params.puff_count] {
        let wall_radius = params.wall_radius_sq(puff[1] / params.height).sqrt();
        let outer_reach = (puff[0] * puff[0] + puff[2] * puff[2]).sqrt() + puff[3] - wall_radius;
        assert!(
            extent >= outer_reach,
            "extent {extent} must reach puff edge {outer_reach}"
        );
    }
}

#[test]
fn glsl_shadow_volume_extents_match_the_rust_constants() {
    let source = wind_glsl_source("shadow_volume.glsl");
    assert_eq!(
        glsl_int_constant(&source, "WIND_SHADOW_RADIAL"),
        WIND_SHADOW_VOLUME_RADIAL as i64
    );
    assert_eq!(
        glsl_int_constant(&source, "WIND_SHADOW_HEIGHT"),
        WIND_SHADOW_VOLUME_HEIGHT as i64
    );
    assert_eq!(
        glsl_int_constant(&source, "WIND_SHADOW_THETA"),
        WIND_SHADOW_VOLUME_THETA as i64
    );
    assert_eq!(
        glsl_int_constant(&source, "WIND_SHADOW_SLOTS"),
        WIND_SHADOW_VOLUME_SLOTS as i64
    );
}

#[test]
fn glsl_polynomial_terms_match_the_rust_mirror_and_cover_the_piece_degree() {
    let source = wind_glsl_source("shell_integral.glsl");

    assert_eq!(
        glsl_int_constant(&source, "WIND_POLY_TERMS"),
        POLY_TERMS as i64
    );
    assert_eq!(
        glsl_int_constant(&source, "WIND_MODULATION_CELLS"),
        MODULATION_CELLS as i64
    );
    assert_eq!(
        glsl_int_constant(&source, "WIND_ACTIVE_CELLS_MIN"),
        ACTIVE_CELLS_MIN as i64
    );
    assert_eq!(
        glsl_int_constant(
            &wind_glsl_source("shell_field.glsl"),
            "WIND_EDDY_OCTAVE_COUNT"
        ),
        EDDY_OCTAVE_COUNT as i64
    );
    assert_eq!(
        glsl_int_constant(&source, "WIND_MAX_KNOTS"),
        WIND_MAX_KNOTS as i64
    );
    assert_eq!(
        glsl_int_constant(&source, "WIND_PUFFS_PER_RAY"),
        WIND_PUFFS_PER_RAY as i64
    );

    let biweight_of_quadratic_degree = 8;
    let envelope_degree = 3;
    let streak_degree = 1;
    let piece_degree = biweight_of_quadratic_degree + envelope_degree + streak_degree;
    assert!(
        piece_degree < POLY_TERMS,
        "a piece polynomial of degree {piece_degree} needs {} terms",
        piece_degree + 1
    );
}
