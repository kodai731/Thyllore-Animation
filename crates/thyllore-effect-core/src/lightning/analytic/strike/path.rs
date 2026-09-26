use super::segment::{compute_distance, push_segment, Segment};
use crate::lightning::effect::LIGHTNING_MAX_WAYPOINTS;
use crate::LightningEffect;
use cgmath::{InnerSpace, Quaternion, Rad, Rotation, Rotation3, Vector3};
use thyllore_math_core::{hash_f32, hash_u32};

pub(super) fn perpendicular_basis(dir: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let axis = Vector3::from(dir);
    let axis = if axis.magnitude2() > 0.0 {
        axis.normalize()
    } else {
        Vector3::unit_z()
    };

    let canonical_down = -Vector3::unit_y();
    let u = if axis.dot(canonical_down) < -1.0 + 1e-6 {
        Vector3::unit_x()
    } else {
        Quaternion::from_arc(canonical_down, axis, None).rotate_vector(Vector3::unit_x())
    };

    let v = axis.cross(u);
    (u.into(), v.into())
}

pub(super) fn scatter_end_offset(
    end_offset: [f32; 3],
    variance: f32,
    azimuth_hash: f32,
    radius_hash: f32,
) -> [f32; 3] {
    let (u, v) = perpendicular_basis(end_offset);
    let azimuth = azimuth_hash * std::f32::consts::TAU;
    let radius = variance * radius_hash.sqrt();
    let offset_x = radius * azimuth.cos();
    let offset_y = radius * azimuth.sin();
    [
        end_offset[0] + u[0] * offset_x + v[0] * offset_y,
        end_offset[1] + u[1] * offset_x + v[1] * offset_y,
        end_offset[2] + u[2] * offset_x + v[2] * offset_y,
    ]
}

pub(super) fn rotate_around_axis(v: [f32; 3], axis: [f32; 3], angle: f32) -> [f32; 3] {
    let axis = Vector3::from(axis);
    if axis.magnitude2() <= 0.0 {
        return v;
    }

    let rotation = Quaternion::from_axis_angle(axis.normalize(), Rad(angle));
    rotation.rotate_vector(Vector3::from(v)).into()
}

pub(super) fn compute_branch_tangent(
    parent_tangent: [f32; 3],
    branch_angle: f32,
    azimuth_hash: f32,
    angle_hash: f32,
) -> [f32; 3] {
    let (u, v) = perpendicular_basis(parent_tangent);
    let azimuth = azimuth_hash * std::f32::consts::TAU;
    let axis = Vector3::from(u) * azimuth.cos() + Vector3::from(v) * azimuth.sin();

    let angle = branch_angle * (0.5 + angle_hash);
    rotate_around_axis(parent_tangent, axis.into(), angle)
}

pub(super) fn displace_path(
    effect: &LightningEffect,
    seed: u32,
    reseed: u32,
    strike: u32,
    start: [f32; 3],
    end: [f32; 3],
    levels: u32,
) -> Vec<[f32; 3]> {
    if levels == 0 {
        return vec![start, end];
    }

    let chord = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
    let c = (chord[0] * chord[0] + chord[1] * chord[1] + chord[2] * chord[2]).sqrt();
    let roughness = effect.shape.roughness;
    let tortuosity = effect.shape.tortuosity;

    let mut points: Vec<[f32; 3]> = vec![start, end];

    for l in 1..=levels {
        let mut next_points: Vec<[f32; 3]> = Vec::with_capacity(1 << (l + 1));

        for i in 0..points.len() - 1 {
            let a = points[i];
            let b = points[i + 1];

            let mid = [
                (a[0] + b[0]) * 0.5,
                (a[1] + b[1]) * 0.5,
                (a[2] + b[2]) * 0.5,
            ];

            let (u, v) = perpendicular_basis(chord);

            let node = i as u32;
            let mut hash_base: [u32; 5] = [seed, strike, l, node, 0];
            let mut hash_base_angle: [u32; 5] = [seed, strike, l, node, 1];

            let (dist_hash, angle_hash) = if l >= effect.timing.reseed_level {
                let mut extended_dist: [u32; 6] = [0; 6];
                extended_dist[..5].copy_from_slice(&hash_base);
                extended_dist[5] = reseed;
                let mut extended_angle: [u32; 6] = [0; 6];
                extended_angle[..5].copy_from_slice(&hash_base_angle);
                extended_angle[5] = reseed;
                (hash_f32(&extended_dist), hash_f32(&extended_angle))
            } else {
                (hash_f32(&hash_base), hash_f32(&hash_base_angle))
            };

            let displacement_amount =
                tortuosity * c * roughness.powi(l as i32) * (0.5 + 0.5 * dist_hash);

            let angle = 2.0 * std::f32::consts::PI * angle_hash;

            let displaced = [
                mid[0] + displacement_amount * (angle.cos() * u[0] + angle.sin() * v[0]),
                mid[1] + displacement_amount * (angle.cos() * u[1] + angle.sin() * v[1]),
                mid[2] + displacement_amount * (angle.cos() * u[2] + angle.sin() * v[2]),
            ];

            next_points.push(a);
            next_points.push(displaced);
        }
        next_points.push(points.last().unwrap().clone());
        points = next_points;
    }

    points
}

pub(super) fn emit_path(
    segments: &mut Vec<Segment>,
    points: &[[f32; 3]],
    r_start: f32,
    r_end: f32,
    intensity: f32,
    arrival_origin: f32,
) {
    if points.len() < 2 {
        return;
    }

    let n = points.len() - 1;
    let mut travelled = arrival_origin;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let radius = r_start * (1.0 - t) + r_end * t;
        let seg_len = compute_distance(points[i], points[i + 1]);

        if !push_segment(
            segments,
            Segment {
                a: points[i],
                b: points[i + 1],
                r0: radius,
                r1: radius,
                intensity,
                arrival_start: travelled,
                arrival_end: travelled + seg_len,
            },
        ) {
            break;
        }
        travelled += seg_len;
    }
}

pub(super) fn branch_detail_levels(
    budget: usize,
    length: f32,
    total_length: f32,
    max_levels: u32,
) -> u32 {
    let segment_share = budget as f32 * length / total_length;
    let levels = segment_share.log2().floor().max(1.0) as u32;
    levels.min(max_levels)
}

pub(super) fn build_waypoint_path(
    effect: &LightningEffect,
    seed: u32,
    reseed: u32,
    strike: u32,
    start: [f32; 3],
    end: [f32; 3],
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let waypoint_count = (effect.shape.waypoint_count as usize).min(LIGHTNING_MAX_WAYPOINTS);
    let mut corners = vec![start];
    corners.extend_from_slice(&effect.shape.waypoints[..waypoint_count]);
    corners.push(end);

    let total_length: f32 = corners
        .windows(2)
        .map(|pair| compute_distance(pair[0], pair[1]))
        .sum();

    let mut points = vec![start];
    let mut interval_ends = Vec::new();
    for (interval, pair) in corners.windows(2).enumerate() {
        let levels = if total_length > 0.0 {
            branch_detail_levels(
                1 << effect.shape.detail_levels,
                compute_distance(pair[0], pair[1]),
                total_length,
                effect.shape.detail_levels,
            )
        } else {
            effect.shape.detail_levels
        };
        let stream = if interval == 0 {
            strike
        } else {
            hash_u32(&[strike, interval as u32])
        };

        let interval_points = displace_path(effect, seed, reseed, stream, pair[0], pair[1], levels);
        interval_ends.extend(std::iter::repeat(pair[1]).take(interval_points.len() - 1));
        points.extend_from_slice(&interval_points[1..]);
    }

    (points, interval_ends)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{difference, dot, length, waypoint_effect, WAYPOINTS};
    use super::*;

    #[test]
    fn test_perpendicular_basis_is_orthonormal() {
        let directions = [
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 3.0],
            [0.3, 0.6, -0.2],
        ];

        for dir in directions {
            let (u, v) = perpendicular_basis(dir);
            assert!((length(u) - 1.0).abs() < 1e-5, "{dir:?} gave {u:?}");
            assert!((length(v) - 1.0).abs() < 1e-5, "{dir:?} gave {v:?}");
            assert!(dot(u, v).abs() < 1e-5, "{dir:?} gave {u:?} {v:?}");
            assert!(dot(u, dir).abs() < 1e-5, "{dir:?} gave {u:?}");
            assert!(dot(v, dir).abs() < 1e-5, "{dir:?} gave {v:?}");
        }
    }

    #[test]
    fn test_perpendicular_basis_is_deterministic() {
        let first = perpendicular_basis([0.2, 0.7, -0.4]);
        let second = perpendicular_basis([0.2, 0.7, -0.4]);
        assert_eq!(first, second);
    }

    #[test]
    fn test_perpendicular_basis_continuity_xy_tilt() {
        let mut prev_u: Option<[f32; 3]> = None;
        for i in 0..=150 {
            let angle_deg = 20.0 + i as f32 * 0.1;
            let angle_rad = angle_deg.to_radians();
            let dir = [angle_rad.sin(), -angle_rad.cos(), 0.0];
            let (u, _) = perpendicular_basis(dir);
            if let Some(prev) = prev_u {
                let diff = difference(u, prev);
                let diff_norm = length(diff);
                assert!(
                    diff_norm < 0.01,
                    "discontinuity at {}°: |u_i - u_{{i-1}}| = {:.6}",
                    angle_deg,
                    diff_norm
                );
            }
            prev_u = Some(u);
        }
    }

    #[test]
    fn test_rotate_around_axis_quarter_turn() {
        let rotated = rotate_around_axis(
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            std::f32::consts::FRAC_PI_2,
        );
        assert!((rotated[0]).abs() < 1e-6, "{rotated:?}");
        assert!((rotated[1] - 1.0).abs() < 1e-6, "{rotated:?}");
        assert!((rotated[2]).abs() < 1e-6, "{rotated:?}");
    }

    #[test]
    fn test_rotate_around_axis_preserves_length_and_axis_component() {
        let axis = [0.0, 1.0, 0.0];
        let v = [0.5, 2.0, -1.0];
        let rotated = rotate_around_axis(v, axis, 1.234);
        assert!((length(rotated) - length(v)).abs() < 1e-5, "{rotated:?}");
        assert!(
            (dot(rotated, axis) - dot(v, axis)).abs() < 1e-5,
            "{rotated:?}"
        );
    }

    #[test]
    fn test_rotate_around_axis_ignores_degenerate_axis() {
        assert_eq!(
            rotate_around_axis([1.0, 2.0, 3.0], [0.0; 3], 0.7),
            [1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn test_compute_branch_tangent_angle_within_bounds() {
        let parent = [0.0, 1.0, 0.0];
        let branch_angle = 0.4;

        let test_cases: &[(f32, f32)] =
            &[(0.1, 0.2), (0.3, 0.5), (0.5, 0.8), (0.7, 0.1), (0.9, 0.9)];

        for (azimuth_hash, angle_hash) in test_cases {
            let result = compute_branch_tangent(parent, branch_angle, *azimuth_hash, *angle_hash);

            let result_len = length(result);
            let parent_len = length(parent);
            let cos_theta = dot(result, parent) / (result_len * parent_len);
            let theta = cos_theta.acos();

            let min_angle = 0.5 * branch_angle;
            let max_angle = 1.5 * branch_angle;
            assert!(
                theta >= min_angle - 1e-4 && theta <= max_angle + 1e-4,
                "azimuth={:.1} angle_hash={:.1}: theta={:.4}, expected [{:.4}, {:.4}]",
                azimuth_hash,
                angle_hash,
                theta,
                min_angle,
                max_angle
            );
        }
    }

    #[test]
    fn test_scatter_end_offset_stays_within_perpendicular_disk() {
        let end_offset = [0.3, -5.0, 1.2];
        let variance = 2.5;
        for azimuth_hash in [0.0, 0.2, 0.55, 0.99] {
            for radius_hash in [0.0, 0.3, 1.0] {
                let scattered = scatter_end_offset(end_offset, variance, azimuth_hash, radius_hash);
                let shift = difference(scattered, end_offset);
                assert!(dot(shift, end_offset).abs() < 1e-4, "{shift:?}");
                assert!(length(shift) <= variance + 1e-5, "{shift:?}");
            }
        }
    }

    #[test]
    fn test_waypoint_path_passes_through_every_waypoint() {
        for count in 1..=3 {
            let effect = waypoint_effect(&WAYPOINTS[..count]);
            let (points, interval_ends) =
                build_waypoint_path(&effect, 42, 7, 0, [0.0; 3], effect.shape.end_offset);

            for waypoint in &WAYPOINTS[..count] {
                assert!(
                    points.contains(waypoint),
                    "{count} waypoints miss {waypoint:?}"
                );
            }
            assert_eq!(points.first(), Some(&[0.0; 3]));
            assert_eq!(points.last(), Some(&effect.shape.end_offset));
            assert_eq!(interval_ends.len(), points.len() - 1);
        }
    }

    #[test]
    fn test_waypoint_path_stays_within_the_segment_budget() {
        for count in 0..=3 {
            let effect = waypoint_effect(&WAYPOINTS[..count]);
            let (points, _) =
                build_waypoint_path(&effect, 42, 7, 0, [0.0; 3], effect.shape.end_offset);
            assert!(
                points.len() - 1 <= 1 << effect.shape.detail_levels,
                "{count} waypoints give {} segments",
                points.len() - 1
            );
        }
    }

    #[test]
    fn test_branch_detail_levels_follows_the_length_share_of_the_budget() {
        assert_eq!(branch_detail_levels(128, 1.0, 16.0, 6), 3);
        assert_eq!(branch_detail_levels(128, 3.0, 16.0, 6), 4);
        assert_eq!(branch_detail_levels(128, 16.0, 16.0, 6), 6);
        assert_eq!(branch_detail_levels(128, 0.1, 16.0, 6), 1);
        assert_eq!(branch_detail_levels(0, 1.0, 16.0, 6), 1);
    }
}
