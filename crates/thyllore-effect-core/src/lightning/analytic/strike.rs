use crate::lightning::analytic::hash::{hash_f32, HashChannel};
use crate::lightning::analytic::timing;
use crate::lightning::effect::LightningSource;
use crate::LightningEffect;
use cgmath::{InnerSpace, Quaternion, Rad, Rotation, Rotation3, Vector3};

pub const LIGHTNING_MAX_SEGMENTS: usize = 256;

fn push_segment(segments: &mut Vec<Segment>, seg: Segment) -> bool {
    if segments.len() >= LIGHTNING_MAX_SEGMENTS {
        return false;
    }
    segments.push(seg);
    true
}

#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub r0: f32,
    pub r1: f32,
    pub intensity: f32,
}

pub fn build_lightning_segments(effect: &LightningEffect, t: f32) -> Vec<Segment> {
    let (k, tau) = match timing::active_burst(effect, t) {
        Some(v) => v,
        None => return Vec::new(),
    };

    let intensity =
        timing::stroke_intensity(effect, tau) * timing::flicker_factor(effect, effect.seed, k, tau);
    if intensity <= 0.0 {
        return Vec::new();
    }

    let burst_seed = crate::lightning::hash_u32(&[effect.seed, k]);
    let reseed = timing::reseed_index(effect, tau);

    let mut charged = effect.clone();
    if let LightningSource::Shell { .. } = effect.source {
        charged.strikes_per_burst = timing::charge_alive_strikes(effect, tau);
    }

    let mut segments = build_strike_segments(&charged, burst_seed, reseed);

    for seg in &mut segments {
        seg.intensity *= intensity;
    }

    segments
}

fn perpendicular_basis(dir: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let axis = Vector3::from(dir);
    let axis = if axis.magnitude2() > 0.0 {
        axis.normalize()
    } else {
        Vector3::unit_z()
    };

    let reference = if axis.y.abs() < 0.9 {
        Vector3::unit_y()
    } else {
        Vector3::unit_x()
    };

    let u = reference.cross(axis).normalize();
    let v = axis.cross(u);
    (u.into(), v.into())
}

fn rotate_around_axis(v: [f32; 3], axis: [f32; 3], angle: f32) -> [f32; 3] {
    let axis = Vector3::from(axis);
    if axis.magnitude2() <= 0.0 {
        return v;
    }

    let rotation = Quaternion::from_axis_angle(axis.normalize(), Rad(angle));
    rotation.rotate_vector(Vector3::from(v)).into()
}

fn displace_path(
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
    let roughness = effect.roughness;
    let tortuosity = effect.tortuosity;

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

            let (dist_hash, angle_hash) = if l >= effect.reseed_level {
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

fn emit_path(
    segments: &mut Vec<Segment>,
    points: &[[f32; 3]],
    r_start: f32,
    r_end: f32,
    intensity: f32,
) {
    if points.len() < 2 {
        return;
    }

    let n = points.len() - 1;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let radius = r_start * (1.0 - t) + r_end * t;

        if !push_segment(
            segments,
            Segment {
                a: points[i],
                b: points[i + 1],
                r0: radius,
                r1: radius,
                intensity,
            },
        ) {
            break;
        }
    }
}

fn spawn_branches(
    segments: &mut Vec<Segment>,
    effect: &LightningEffect,
    seed: u32,
    reseed: u32,
    depth: u32,
) {
    let main_count = segments.len();

    for i in 0..main_count {
        if segments.len() >= LIGHTNING_MAX_SEGMENTS {
            break;
        }

        let seg = &segments[i];
        let progress = i as f32 / main_count as f32;
        let is_end_zone = progress >= 0.9;

        let mid_pos: [f32; 3] = [
            (seg.a[0] + seg.b[0]) * 0.5,
            (seg.a[1] + seg.b[1]) * 0.5,
            (seg.a[2] + seg.b[2]) * 0.5,
        ];

        let chord_remaining_len = [
            seg.b[0] - mid_pos[0],
            seg.b[1] - mid_pos[1],
            seg.b[2] - mid_pos[2],
        ];
        let chord_remaining_len = (chord_remaining_len[0] * chord_remaining_len[0]
            + chord_remaining_len[1] * chord_remaining_len[1]
            + chord_remaining_len[2] * chord_remaining_len[2])
            .sqrt();

        let hash_value = hash_f32(&[seed, reseed, i as u32, depth]);
        let effective_probability = if is_end_zone {
            effect.branch_probability * 2.0
        } else {
            effect.branch_probability
        };

        if hash_value < effective_probability {
            let parent_tangent: [f32; 3] = [
                seg.b[0] - seg.a[0],
                seg.b[1] - seg.a[1],
                seg.b[2] - seg.a[2],
            ];

            let axis_x = hash_f32(&[seed, reseed, i as u32, depth, 0]);
            let axis_y = hash_f32(&[seed, reseed, i as u32, depth, 1]);
            let axis_z = hash_f32(&[seed, reseed, i as u32, depth, 2]);
            let rotation_axis = [axis_x * 2.0 - 1.0, axis_y * 2.0 - 1.0, axis_z * 2.0 - 1.0];

            let angle_raw = hash_f32(&[seed, reseed, i as u32, depth, 3]);
            let mut angle = angle_raw * std::f32::consts::PI;
            if is_end_zone {
                angle *= 1.5;
            }

            let child_tangent = rotate_around_axis(parent_tangent, rotation_axis, angle);
            let child_length = effect.branch_length_ratio * chord_remaining_len;

            let tangent_mag = (child_tangent[0] * child_tangent[0]
                + child_tangent[1] * child_tangent[1]
                + child_tangent[2] * child_tangent[2])
                .sqrt();
            let child_end: [f32; 3] = if tangent_mag > 0.0 {
                let scale = child_length / tangent_mag;
                [
                    mid_pos[0] + child_tangent[0] * scale,
                    mid_pos[1] + child_tangent[1] * scale,
                    mid_pos[2] + child_tangent[2] * scale,
                ]
            } else {
                mid_pos
            };

            let child_levels = effect.detail_levels.saturating_sub(1);
            let child_points = displace_path(
                effect,
                seed,
                reseed,
                i as u32,
                mid_pos,
                child_end,
                child_levels,
            );

            let mut child_segments = Vec::new();
            let child_radius_start = effect.core_radius * effect.branch_radius_ratio;
            let child_radius_end = child_radius_start * effect.tip_radius_ratio;
            let child_intensity = effect.core_intensity * effect.branch_intensity_ratio;

            emit_path(
                &mut child_segments,
                &child_points,
                child_radius_start,
                child_radius_end,
                child_intensity,
            );

            if depth < effect.branch_depth {
                spawn_branches(&mut child_segments, effect, seed, reseed, depth + 1);
            }

            for seg in child_segments {
                if !push_segment(segments, seg) {
                    break;
                }
            }
        }
    }
}

pub fn build_strike_segments(effect: &LightningEffect, seed: u32, reseed: u32) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();

    if effect.beam_radius > 0.0 {
        let end_offset = effect.end_offset;
        let beam_radius = effect.beam_radius;
        let tip_radius_ratio = effect.tip_radius_ratio;
        let arc_count = effect.beam_arc_count as usize;

        if !push_segment(
            &mut segments,
            Segment {
                a: [0.0, 0.0, 0.0],
                b: end_offset,
                r0: beam_radius,
                r1: beam_radius * tip_radius_ratio,
                intensity: effect.core_intensity,
            },
        ) {
            return segments;
        }

        for i in 0..arc_count {
            let t = (i as f32 + 0.5) / arc_count as f32;
            let node_pos: [f32; 3] = [end_offset[0] * t, end_offset[1] * t, end_offset[2] * t];

            let parent_tangent: [f32; 3] = [end_offset[0], end_offset[1], end_offset[2]];

            let axis_x = hash_f32(&[seed, reseed, i as u32, HashChannel::BranchAxis.into(), 0]);
            let axis_y = hash_f32(&[seed, reseed, i as u32, HashChannel::BranchAxis.into(), 1]);
            let axis_z = hash_f32(&[seed, reseed, i as u32, HashChannel::BranchAxis.into(), 2]);
            let rotation_axis = [axis_x * 2.0 - 1.0, axis_y * 2.0 - 1.0, axis_z * 2.0 - 1.0];

            let angle = hash_f32(&[seed, reseed, i as u32, HashChannel::BranchAngle.into()])
                * 2.0
                * std::f32::consts::PI;

            let child_tangent = rotate_around_axis(parent_tangent, rotation_axis, angle);

            let chord_len = (end_offset[0] * end_offset[0]
                + end_offset[1] * end_offset[1]
                + end_offset[2] * end_offset[2])
                .sqrt();
            let child_length = effect.branch_length_ratio * chord_len;

            let tangent_mag = (child_tangent[0] * child_tangent[0]
                + child_tangent[1] * child_tangent[1]
                + child_tangent[2] * child_tangent[2])
                .sqrt();
            let child_end: [f32; 3] = if tangent_mag > 0.0 {
                let scale = child_length / tangent_mag;
                [
                    node_pos[0] + child_tangent[0] * scale,
                    node_pos[1] + child_tangent[1] * scale,
                    node_pos[2] + child_tangent[2] * scale,
                ]
            } else {
                node_pos
            };

            let child_levels = effect.detail_levels.saturating_sub(1);
            let child_points = displace_path(
                effect,
                seed,
                reseed,
                i as u32,
                node_pos,
                child_end,
                child_levels,
            );

            let child_radius_start = beam_radius * effect.branch_radius_ratio;
            let child_radius_end = child_radius_start * tip_radius_ratio;
            let child_intensity = effect.core_intensity * effect.branch_intensity_ratio;

            emit_path(
                &mut segments,
                &child_points,
                child_radius_start,
                child_radius_end,
                child_intensity,
            );
        }
    } else {
        let end_offset = effect.end_offset;
        let strikes = match &effect.source {
            LightningSource::Point => 1u32,
            LightningSource::Shell { radius } => {
                let mut strikes = effect.strikes_per_burst as usize;
                if strikes > LIGHTNING_MAX_SEGMENTS {
                    strikes = LIGHTNING_MAX_SEGMENTS;
                }
                strikes as u32
            }
        };

        for s in 0..strikes {
            let start: [f32; 3] = match &effect.source {
                LightningSource::Point => [0.0, 0.0, 0.0],
                LightningSource::Shell { radius } => {
                    let dir_x =
                        hash_f32(&[seed, s, HashChannel::SourcePoint.into(), 0]) * 2.0 - 1.0;
                    let dir_y =
                        hash_f32(&[seed, s, HashChannel::SourcePoint.into(), 1]) * 2.0 - 1.0;
                    let dir_z =
                        hash_f32(&[seed, s, HashChannel::SourcePoint.into(), 2]) * 2.0 - 1.0;
                    let dir_len = (dir_x * dir_x + dir_y * dir_y + dir_z * dir_z).sqrt();
                    if dir_len > 0.0 {
                        [
                            end_offset[0] + radius * dir_x / dir_len,
                            end_offset[1] + radius * dir_y / dir_len,
                            end_offset[2] + radius * dir_z / dir_len,
                        ]
                    } else {
                        end_offset
                    }
                }
            };

            let detail_levels = effect.detail_levels.min(3);
            let points = displace_path(effect, seed, reseed, s, start, end_offset, detail_levels);

            emit_path(
                &mut segments,
                &points,
                effect.core_radius,
                effect.core_radius * effect.tip_radius_ratio,
                effect.core_intensity,
            );

            if segments.len() >= LIGHTNING_MAX_SEGMENTS {
                break;
            }

            spawn_branches(&mut segments, effect, seed, reseed, 0);

            if segments.len() >= LIGHTNING_MAX_SEGMENTS {
                break;
            }
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    fn length(v: [f32; 3]) -> f32 {
        dot(v, v).sqrt()
    }

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

    fn segments_match(a: &[Segment], b: &[Segment]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        for (sa, sb) in a.iter().zip(b.iter()) {
            if sa.a[0].to_bits() != sb.a[0].to_bits()
                || sa.a[1].to_bits() != sb.a[1].to_bits()
                || sa.a[2].to_bits() != sb.a[2].to_bits()
                || sa.b[0].to_bits() != sb.b[0].to_bits()
                || sa.b[1].to_bits() != sb.b[1].to_bits()
                || sa.b[2].to_bits() != sb.b[2].to_bits()
                || sa.r0.to_bits() != sb.r0.to_bits()
                || sa.r1.to_bits() != sb.r1.to_bits()
                || sa.intensity.to_bits() != sb.intensity.to_bits()
            {
                return false;
            }
        }
        true
    }

    #[test]
    fn test_bit_parity() {
        let effect = LightningEffect::default();
        let seed = 42u32;
        let reseed = 7u32;
        let first = build_strike_segments(&effect, seed, reseed);
        let second = build_strike_segments(&effect, seed, reseed);
        assert!(
            segments_match(&first, &second),
            "same (seed, reseed) must be bit-identical"
        );
    }

    #[test]
    fn test_seed_variance() {
        let effect = LightningEffect::default();
        let first = build_strike_segments(&effect, 0, 0);
        let second = build_strike_segments(&effect, 1, 0);
        assert!(
            !segments_match(&first, &second),
            "different seeds must differ"
        );
    }

    #[test]
    fn test_max_segments_limit() {
        let mut effect = LightningEffect::default();
        effect.detail_levels = 7;
        effect.branch_probability = 1.0;
        effect.branch_depth = 3;
        let first = build_strike_segments(&effect, 123, 456);
        let second = build_strike_segments(&effect, 123, 456);
        assert!(
            first.len() <= LIGHTNING_MAX_SEGMENTS,
            "len {} exceeds {}",
            first.len(),
            LIGHTNING_MAX_SEGMENTS
        );
        assert_eq!(first.len(), second.len(), "same seed must give same length");
    }

    #[test]
    fn test_point_source_endpoints() {
        let mut effect = LightningEffect::default();
        effect.source = LightningSource::Point;
        effect.end_offset = [3.0, 4.0, 5.0];
        effect.branch_probability = 0.0;

        let segments = build_strike_segments(&effect, 0, 0);

        assert!(!segments.is_empty(), "Point source must produce segments");
        assert_eq!(segments[0].a, [0.0, 0.0, 0.0], "first point must be origin");

        let main_path_end = segments
            .iter()
            .find(|s| {
                (s.b[0] - effect.end_offset[0]).abs() < 1e-3
                    && (s.b[1] - effect.end_offset[1]).abs() < 1e-3
                    && (s.b[2] - effect.end_offset[2]).abs() < 1e-3
            })
            .map(|s| s.b)
            .expect("Main path should end at end_offset");
        for axis in 0..3 {
            assert!(
                (main_path_end[axis] - effect.end_offset[axis]).abs() < 1e-6,
                "main path ends at {main_path_end:?}, expected {:?}",
                effect.end_offset
            );
        }
    }

    #[test]
    fn test_shell_source_radius() {
        let mut effect = LightningEffect::default();
        let radius = 2.0;
        effect.source = LightningSource::Shell { radius };
        effect.strikes_per_burst = 5;
        effect.detail_levels = 0;
        effect.branch_probability = 0.0;

        let segments = build_strike_segments(&effect, 0, 0);

        assert_eq!(
            segments.len(),
            effect.strikes_per_burst as usize,
            "one straight segment per strike"
        );
        for segment in &segments {
            let offset_from_end = [
                segment.a[0] - effect.end_offset[0],
                segment.a[1] - effect.end_offset[1],
                segment.a[2] - effect.end_offset[2],
            ];
            assert!(
                (length(offset_from_end) - radius).abs() < 1e-3,
                "strike start {:?} is {} from B, expected {radius}",
                segment.a,
                length(offset_from_end)
            );
        }
    }

    #[test]
    fn test_shell_charge_ramp_limits_alive_strikes() {
        let mut effect = LightningEffect::default();
        effect.source = LightningSource::Shell { radius: 2.0 };
        effect.strikes_per_burst = 8;
        effect.detail_levels = 0;
        effect.branch_probability = 0.0;
        effect.charge_ramp = effect.sustain_time;

        let tau = effect.attack_time + effect.sustain_time * 0.5;
        effect.time = timing::burst_start_time(&effect, 0) + tau;

        let alive = timing::charge_alive_strikes(&effect, tau);
        assert!(alive > 0 && alive < effect.strikes_per_burst, "{alive}");
        assert_eq!(
            build_lightning_segments(&effect, effect.time).len(),
            alive as usize,
            "one straight segment per alive strike"
        );
    }

    #[test]
    fn test_no_segments_outside_a_burst() {
        let effect = LightningEffect::default();
        let before_burst = timing::burst_start_time(&effect, 0) - 1.0;
        assert!(build_lightning_segments(&effect, before_burst).is_empty());
    }

    #[test]
    fn test_beam_mode_first_segment() {
        let mut effect = LightningEffect::default();
        effect.beam_radius = 0.1;
        effect.end_offset = [1.0, 2.0, 3.0];
        let segments = build_strike_segments(&effect, 0, 0);
        assert!(!segments.is_empty(), "beam mode must produce segments");
        assert_eq!(
            segments[0].a,
            [0.0, 0.0, 0.0],
            "first segment a must be origin"
        );
        assert_eq!(
            segments[0].b, effect.end_offset,
            "first segment b must be end_offset"
        );
    }

    #[test]
    fn test_default_effect_never_exceeds_max_segments() {
        let effect = LightningEffect::default();
        for reseed in 0..50u32 {
            let segments =
                build_strike_segments(&effect, crate::lightning::hash_u32(&[0, 0]), reseed);
            assert!(
                segments.len() <= LIGHTNING_MAX_SEGMENTS,
                "reseed {} produced {} segments",
                reseed,
                segments.len()
            );
        }
    }

    #[test]
    fn test_build_lightning_segments_never_exceeds_max() {
        let effect = LightningEffect::default();
        for t in [0.02f32, 0.045] {
            let segments = build_lightning_segments(&effect, t);
            assert!(
                segments.len() <= LIGHTNING_MAX_SEGMENTS,
                "t={} produced {} segments",
                t,
                segments.len()
            );
        }
    }
}
