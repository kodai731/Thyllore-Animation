use super::{build_strike_segments, path::build_waypoint_path, Segment};
use crate::lightning::effect::LightningSource;
use crate::LightningEffect;
use thyllore_math_core::hash_u32;

pub(super) fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(super) fn length(v: [f32; 3]) -> f32 {
    dot(v, v).sqrt()
}

pub(super) fn difference(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(super) fn segments_match(a: &[Segment], b: &[Segment]) -> bool {
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

pub(super) fn straight_counted_branch_effect(branch_count: f32) -> LightningEffect {
    let mut effect = LightningEffect::default();
    effect.shape.detail_levels = 4;
    effect.shape.tortuosity = 0.0;
    effect.branch.depth = 0;
    effect.branch.count = branch_count;
    effect.shape.end_offset = [3.0, -8.0, 1.0];
    effect
}

pub(super) fn build_root_branch_paths(effect: &LightningEffect) -> Vec<Vec<Segment>> {
    let seed = hash_u32(&[0, 0]);
    let segments = build_strike_segments(effect, seed, 0);
    let (main_points, _) =
        build_waypoint_path(effect, seed, 0, 0, [0.0; 3], effect.shape.end_offset);
    let branch_segments = &segments[main_points.len() - 1..];

    let mut paths: Vec<Vec<Segment>> = Vec::new();
    for (i, seg) in branch_segments.iter().enumerate() {
        let continues_previous =
            i > 0 && length(difference(seg.a, branch_segments[i - 1].b)) <= 1e-6;
        match paths.last_mut() {
            Some(path) if continues_previous => path.push(*seg),
            _ => paths.push(vec![*seg]),
        }
    }
    paths
}

pub(super) fn waypoint_effect(waypoints: &[[f32; 3]]) -> LightningEffect {
    let mut effect = LightningEffect::default();
    effect.shape.source = LightningSource::Point;
    effect.shape.end_offset = [3.0, -8.0, 1.0];
    effect.shape.waypoints[..waypoints.len()].copy_from_slice(waypoints);
    effect.shape.waypoint_count = waypoints.len() as u32;
    effect
}

pub(super) const WAYPOINTS: [[f32; 3]; 3] =
    [[1.5, -2.0, 0.5], [-1.0, -4.5, 1.0], [2.0, -6.0, -0.5]];
