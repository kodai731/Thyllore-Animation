mod branch;
mod path;
mod segment;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub(crate) use segment::segment_length;
pub use segment::{Segment, LIGHTNING_MAX_SEGMENTS};

use crate::lightning::analytic::hash_channel::HashChannel;
use crate::lightning::analytic::timing;
use crate::lightning::effect::LightningSource;
use crate::LightningEffect;
use branch::spawn_branches;
use cgmath::Vector3;
use path::{build_waypoint_path, displace_path, emit_path, rotate_around_axis, scatter_end_offset};
use segment::{clip_to_growth_arrival, push_segment};
use thyllore_math_core::{hash_f32, hash_u32};

pub fn build_lightning_segments(effect: &LightningEffect, t: f32) -> Vec<Segment> {
    let (k, tau) = match timing::active_burst(effect, t) {
        Some(v) => v,
        None => return Vec::new(),
    };

    let intensity = timing::stroke_intensity(effect, tau)
        * timing::flicker_factor(effect, effect.timing.seed, k, tau);
    if intensity <= 0.0 {
        return Vec::new();
    }

    let burst_seed = hash_u32(&[effect.timing.seed, k]);
    let reseed = timing::reseed_index(effect, tau);

    let mut charged = effect.clone();
    if let LightningSource::Shell { .. } = effect.shape.source {
        charged.shape.strikes_per_burst = timing::charge_alive_strikes(effect, tau);
    }

    if effect.shape.end_variance > 0.0 {
        let azimuth_hash = hash_f32(&[effect.timing.seed, k, HashChannel::EndVariance.into(), 0]);
        let radius_hash = hash_f32(&[effect.timing.seed, k, HashChannel::EndVariance.into(), 1]);
        charged.shape.end_offset = scatter_end_offset(
            charged.shape.end_offset,
            effect.shape.end_variance,
            azimuth_hash,
            radius_hash,
        );
    }

    let mut segments = build_strike_segments(&charged, burst_seed, reseed);

    if let LightningSource::Point = effect.shape.source {
        if effect.timing.growth_time > 0.0 {
            let max_arrival = segments
                .iter()
                .map(|s| s.arrival_end)
                .fold(0.0f32, f32::max);
            let front_distance = (tau / effect.timing.growth_time).min(1.0) * max_arrival;
            segments = clip_to_growth_arrival(&segments, front_distance);
        }
    }

    for seg in &mut segments {
        seg.intensity *= intensity;
    }

    segments
}

pub fn build_strike_segments(effect: &LightningEffect, seed: u32, reseed: u32) -> Vec<Segment> {
    if effect.look.beam_radius > 0.0 {
        build_beam_segments(effect, seed, reseed)
    } else {
        build_channel_segments(effect, seed, reseed)
    }
}

fn build_beam_segments(effect: &LightningEffect, seed: u32, reseed: u32) -> Vec<Segment> {
    let mut segments = Vec::new();
    let end_offset = effect.shape.end_offset;
    let beam_radius = effect.look.beam_radius;

    if !push_segment(
        &mut segments,
        Segment {
            a: [0.0, 0.0, 0.0],
            b: end_offset,
            r0: beam_radius,
            r1: beam_radius * effect.shape.tip_radius_ratio,
            intensity: 1.0,
            arrival_start: 0.0,
            arrival_end: 0.0,
        },
    ) {
        return segments;
    }

    let arc_count = effect.look.beam_arc_count as usize;
    for i in 0..arc_count {
        let t = (i as f32 + 0.5) / arc_count as f32;
        let node_pos: [f32; 3] = [end_offset[0] * t, end_offset[1] * t, end_offset[2] * t];
        push_beam_arc(&mut segments, effect, seed, reseed, i as u32, node_pos);
    }

    segments
}

fn push_beam_arc(
    segments: &mut Vec<Segment>,
    effect: &LightningEffect,
    seed: u32,
    reseed: u32,
    arc: u32,
    node_pos: [f32; 3],
) {
    let end_offset = effect.shape.end_offset;
    let axis_x = hash_f32(&[seed, reseed, arc, HashChannel::BranchAxis.into(), 0]);
    let axis_y = hash_f32(&[seed, reseed, arc, HashChannel::BranchAxis.into(), 1]);
    let axis_z = hash_f32(&[seed, reseed, arc, HashChannel::BranchAxis.into(), 2]);
    let rotation_axis = [axis_x * 2.0 - 1.0, axis_y * 2.0 - 1.0, axis_z * 2.0 - 1.0];
    let angle = hash_f32(&[seed, reseed, arc, HashChannel::BranchAngle.into()])
        * 2.0
        * std::f32::consts::PI;
    let child_tangent = rotate_around_axis(end_offset, rotation_axis, angle);

    let chord_len = (end_offset[0] * end_offset[0]
        + end_offset[1] * end_offset[1]
        + end_offset[2] * end_offset[2])
        .sqrt();
    let child_length = effect.branch.length_ratio * chord_len;
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

    let child_levels = effect.shape.detail_levels.saturating_sub(1);
    let child_points = displace_path(effect, seed, reseed, arc, node_pos, child_end, child_levels);
    let child_radius_start = effect.look.beam_radius * effect.branch.radius_ratio;
    let child_radius_end = child_radius_start * effect.shape.tip_radius_ratio;
    emit_path(
        segments,
        &child_points,
        child_radius_start,
        child_radius_end,
        effect.branch.intensity_ratio,
        0.0,
    );
}

fn build_channel_segments(effect: &LightningEffect, seed: u32, reseed: u32) -> Vec<Segment> {
    let mut segments = Vec::new();
    let end_offset = effect.shape.end_offset;
    let strikes = match &effect.shape.source {
        LightningSource::Point => 1u32,
        LightningSource::Shell { .. } => {
            (effect.shape.strikes_per_burst as usize).min(LIGHTNING_MAX_SEGMENTS) as u32
        }
    };

    for s in 0..strikes {
        let start = strike_start_point(effect, seed, s);
        let (points, interval_ends) = match &effect.shape.source {
            LightningSource::Point => {
                build_waypoint_path(effect, seed, reseed, s, start, end_offset)
            }
            LightningSource::Shell { .. } => {
                let points = displace_path(
                    effect,
                    seed,
                    reseed,
                    s,
                    start,
                    end_offset,
                    effect.shape.detail_levels,
                );
                let interval_ends = vec![end_offset; points.len() - 1];
                (points, interval_ends)
            }
        };

        let mut strike_segments = Vec::new();
        emit_path(
            &mut strike_segments,
            &points,
            effect.shape.core_radius,
            effect.shape.core_radius * effect.shape.tip_radius_ratio,
            1.0,
            0.0,
        );
        spawn_branches(&mut strike_segments, &interval_ends, effect, seed, reseed);

        for seg in strike_segments {
            if !push_segment(&mut segments, seg) {
                break;
            }
        }
        if segments.len() >= LIGHTNING_MAX_SEGMENTS {
            break;
        }
    }

    segments
}

fn strike_start_point(effect: &LightningEffect, seed: u32, strike: u32) -> [f32; 3] {
    let end_offset = effect.shape.end_offset;
    match &effect.shape.source {
        LightningSource::Point => [0.0, 0.0, 0.0],
        LightningSource::Shell { radius } => {
            let dir_x = hash_f32(&[seed, strike, HashChannel::SourcePoint.into(), 0]) * 2.0 - 1.0;
            let dir_y = hash_f32(&[seed, strike, HashChannel::SourcePoint.into(), 1]) * 2.0 - 1.0;
            let dir_z = hash_f32(&[seed, strike, HashChannel::SourcePoint.into(), 2]) * 2.0 - 1.0;
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
    }
}

/// Corners of the local axis-aligned box enclosing every rim capsule alive at `t`.
pub fn compute_lightning_segment_aabb(
    effect: &LightningEffect,
    t: f32,
) -> Option<[Vector3<f32>; 8]> {
    let mut bounds: Option<(Vector3<f32>, Vector3<f32>)> = None;
    for seg in build_lightning_segments(effect, t) {
        let rim_radius = seg.r0.max(seg.r1) * effect.look.rim_ratio;
        let rim_extent = Vector3::new(rim_radius, rim_radius, rim_radius);
        for endpoint in [Vector3::from(seg.a), Vector3::from(seg.b)] {
            let low = endpoint - rim_extent;
            let high = endpoint + rim_extent;
            bounds = Some(match bounds {
                Some((min, max)) => (
                    Vector3::new(min.x.min(low.x), min.y.min(low.y), min.z.min(low.z)),
                    Vector3::new(max.x.max(high.x), max.y.max(high.y), max.z.max(high.z)),
                ),
                None => (low, high),
            });
        }
    }

    let (min, max) = bounds?;
    let mut corners = [min; 8];
    for (index, corner) in corners.iter_mut().enumerate() {
        corner.x = if index & 1 == 0 { min.x } else { max.x };
        corner.y = if index & 2 == 0 { min.y } else { max.y };
        corner.z = if index & 4 == 0 { min.z } else { max.z };
    }
    Some(corners)
}
