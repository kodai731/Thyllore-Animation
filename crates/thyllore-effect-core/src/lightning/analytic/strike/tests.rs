use super::construct::*;
use super::path::{displace_path, emit_path, scatter_end_offset};
use super::segment::{Segment, LIGHTNING_MAX_SEGMENTS};
use super::test_support::{difference, length, segments_match, waypoint_effect};
use crate::lightning::analytic::timing;
use crate::lightning::effect::LightningSource;
use crate::LightningEffect;
use cgmath::Vector3;
use thyllore_math_core::hash_u32;

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
    effect.shape.detail_levels = 7;
    effect.branch.probability = 1.0;
    effect.branch.depth = 3;
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
    effect.shape.source = LightningSource::Point;
    effect.shape.end_offset = [3.0, 4.0, 5.0];
    effect.branch.probability = 0.0;

    let segments = build_strike_segments(&effect, 0, 0);

    assert!(!segments.is_empty(), "Point source must produce segments");
    assert_eq!(segments[0].a, [0.0, 0.0, 0.0], "first point must be origin");

    let main_path_end = segments
        .iter()
        .find(|s| {
            (s.b[0] - effect.shape.end_offset[0]).abs() < 1e-3
                && (s.b[1] - effect.shape.end_offset[1]).abs() < 1e-3
                && (s.b[2] - effect.shape.end_offset[2]).abs() < 1e-3
        })
        .map(|s| s.b)
        .expect("Main path should end at end_offset");
    for axis in 0..3 {
        assert!(
            (main_path_end[axis] - effect.shape.end_offset[axis]).abs() < 1e-6,
            "main path ends at {main_path_end:?}, expected {:?}",
            effect.shape.end_offset
        );
    }
}

#[test]
fn test_shell_source_radius() {
    let mut effect = LightningEffect::default();
    let radius = 2.0;
    effect.shape.source = LightningSource::Shell { radius };
    effect.shape.strikes_per_burst = 5;
    effect.shape.detail_levels = 0;
    effect.branch.count = 0.0;

    let segments = build_strike_segments(&effect, 0, 0);

    assert_eq!(
        segments.len(),
        effect.shape.strikes_per_burst as usize,
        "one straight segment per strike"
    );
    for segment in &segments {
        let offset_from_end = [
            segment.a[0] - effect.shape.end_offset[0],
            segment.a[1] - effect.shape.end_offset[1],
            segment.a[2] - effect.shape.end_offset[2],
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
    effect.shape.source = LightningSource::Shell { radius: 2.0 };
    effect.shape.strikes_per_burst = 8;
    effect.shape.detail_levels = 0;
    effect.branch.count = 0.0;
    effect.timing.charge_ramp = effect.timing.sustain_time;

    let tau = effect.timing.attack_time + effect.timing.sustain_time * 0.5;
    effect.time = timing::burst_start_time(&effect, 0) + tau;

    let alive = timing::charge_alive_strikes(&effect, tau);
    assert!(
        alive > 0 && alive < effect.shape.strikes_per_burst,
        "{alive}"
    );
    assert_eq!(
        build_lightning_segments(&effect, effect.time).len(),
        alive as usize,
        "one straight segment per alive strike"
    );
}

fn straight_point_effect(end_variance: f32) -> LightningEffect {
    let mut effect = LightningEffect::default();
    effect.shape.source = LightningSource::Point;
    effect.shape.end_offset = [0.0, -6.0, 0.0];
    effect.shape.detail_levels = 0;
    effect.branch.count = 0.0;
    effect.timing.burst_count = 2;
    effect.shape.end_variance = end_variance;
    effect
}

fn strike_end_at(effect: &LightningEffect, burst_index: u32, sustain_fraction: f32) -> [f32; 3] {
    let tau = effect.timing.attack_time + effect.timing.sustain_time * sustain_fraction;
    let t = timing::burst_start_time(effect, burst_index) + tau;
    let segments = build_lightning_segments(effect, t);
    segments.last().expect("a burst must produce segments").b
}

#[test]
fn test_scatter_end_offset_zero_variance_keeps_end_offset() {
    let end_offset = [0.3, -5.0, 1.2];
    assert_eq!(scatter_end_offset(end_offset, 0.0, 0.4, 0.9), end_offset);

    let effect = straight_point_effect(0.0);
    let end = strike_end_at(&effect, 0, 0.5);
    assert!(
        length(difference(end, effect.shape.end_offset)) < 1e-5,
        "{end:?}"
    );
}

#[test]
fn test_end_variance_changes_per_burst_and_holds_within_a_burst() {
    let effect = straight_point_effect(2.0);

    let early = strike_end_at(&effect, 0, 0.25);
    let late = strike_end_at(&effect, 0, 0.75);
    let next_burst = strike_end_at(&effect, 1, 0.25);

    assert_eq!(early, late, "end point must not move during a discharge");
    assert!(
        length(difference(early, next_burst)) > 1e-4,
        "{early:?} {next_burst:?}"
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
    effect.look.beam_radius = 0.1;
    effect.shape.end_offset = [1.0, 2.0, 3.0];
    let segments = build_strike_segments(&effect, 0, 0);
    assert!(!segments.is_empty(), "beam mode must produce segments");
    assert_eq!(
        segments[0].a,
        [0.0, 0.0, 0.0],
        "first segment a must be origin"
    );
    assert_eq!(
        segments[0].b, effect.shape.end_offset,
        "first segment b must be end_offset"
    );
}

#[test]
fn test_default_effect_never_exceeds_max_segments() {
    let effect = LightningEffect::default();
    for reseed in 0..50u32 {
        let segments = build_strike_segments(&effect, hash_u32(&[0, 0]), reseed);
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

fn time_inside_first_burst(effect: &LightningEffect) -> f32 {
    timing::burst_start_time(effect, 0)
        + effect.timing.attack_time
        + effect.timing.sustain_time * 0.5
}

fn corner_bounds(corners: &[Vector3<f32>; 8]) -> ([f32; 3], [f32; 3]) {
    let mut min = [corners[0].x, corners[0].y, corners[0].z];
    let mut max = min;
    for corner in corners {
        for axis in 0..3 {
            min[axis] = min[axis].min(corner[axis]);
            max[axis] = max[axis].max(corner[axis]);
        }
    }
    (min, max)
}

fn expanded_segment_bounds(effect: &LightningEffect, segments: &[Segment]) -> ([f32; 3], [f32; 3]) {
    let rim_radius = |seg: &Segment| seg.r0.max(seg.r1) * effect.look.rim_ratio;
    let mut min = [0.0; 3];
    let mut max = [0.0; 3];
    for axis in 0..3 {
        min[axis] = segments
            .iter()
            .map(|seg| seg.a[axis].min(seg.b[axis]) - rim_radius(seg))
            .fold(f32::INFINITY, f32::min);
        max[axis] = segments
            .iter()
            .map(|seg| seg.a[axis].max(seg.b[axis]) + rim_radius(seg))
            .fold(f32::NEG_INFINITY, f32::max);
    }
    (min, max)
}

fn assert_bounds_match(actual: ([f32; 3], [f32; 3]), expected: ([f32; 3], [f32; 3])) {
    for axis in 0..3 {
        assert!(
            (actual.0[axis] - expected.0[axis]).abs() < 1e-5,
            "{actual:?} vs {expected:?}"
        );
        assert!(
            (actual.1[axis] - expected.1[axis]).abs() < 1e-5,
            "{actual:?} vs {expected:?}"
        );
    }
}

#[test]
fn test_compute_lightning_segment_aabb_single_segment() {
    let mut effect = LightningEffect::default();
    effect.shape.detail_levels = 0;
    effect.branch.count = 0.0;
    effect.shape.end_offset = [3.0, -8.0, 1.0];
    let t = time_inside_first_burst(&effect);

    let segments = build_lightning_segments(&effect, t);
    assert_eq!(
        segments.len(),
        1,
        "one straight segment without detail or branches"
    );

    let corners = compute_lightning_segment_aabb(&effect, t).expect("one alive segment");
    assert_bounds_match(
        corner_bounds(&corners),
        expanded_segment_bounds(&effect, &segments),
    );
}

#[test]
fn test_compute_lightning_segment_aabb_multiple_segments_with_branches() {
    let mut effect = LightningEffect::default();
    effect.branch.count = 8.0;
    effect.branch.depth = 2;
    let t = time_inside_first_burst(&effect);

    let mut unbranched = effect.clone();
    unbranched.branch.count = 0.0;
    let main_path_len = build_lightning_segments(&unbranched, t).len();
    let segments = build_lightning_segments(&effect, t);
    assert!(segments.len() > main_path_len, "branches must add segments");

    let corners = compute_lightning_segment_aabb(&effect, t).expect("alive segments");
    assert_bounds_match(
        corner_bounds(&corners),
        expanded_segment_bounds(&effect, &segments),
    );
}

#[test]
fn test_compute_lightning_segment_aabb_zero_segments() {
    let effect = LightningEffect::default();
    let before_burst = timing::burst_start_time(&effect, 0) - 1.0;
    assert!(compute_lightning_segment_aabb(&effect, before_burst).is_none());
}

#[test]
fn test_bolt_preset_branches_dont_grow_upwards() {
    let mut effect = LightningEffect::default();
    crate::lightning::apply_lightning_preset(&mut effect, "bolt");
    effect.timing.burst_jitter = 0.0;

    let segments = build_lightning_segments(&effect, 0.3);
    assert!(
        !segments.is_empty(),
        "bolt preset at t=0.3 must produce segments"
    );

    let max_y_above_origin = 0.15
        * (effect.shape.end_offset[0] * effect.shape.end_offset[0]
            + effect.shape.end_offset[1] * effect.shape.end_offset[1]
            + effect.shape.end_offset[2] * effect.shape.end_offset[2])
            .sqrt();
    for seg in &segments {
        assert!(
            seg.a[1] <= max_y_above_origin,
            "segment a y={:.4} exceeds {:.4}",
            seg.a[1],
            max_y_above_origin
        );
        assert!(
            seg.b[1] <= max_y_above_origin,
            "segment b y={:.4} exceeds {:.4}",
            seg.b[1],
            max_y_above_origin
        );
    }
}

#[test]
fn test_no_waypoints_keeps_the_single_displaced_main_path() {
    let mut effect = waypoint_effect(&[]);
    effect.branch.count = 0.0;
    let (seed, reseed) = (42, 7);

    let points = displace_path(
        &effect,
        seed,
        reseed,
        0,
        [0.0; 3],
        effect.shape.end_offset,
        effect.shape.detail_levels,
    );
    let mut expected = Vec::new();
    emit_path(
        &mut expected,
        &points,
        effect.shape.core_radius,
        effect.shape.core_radius * effect.shape.tip_radius_ratio,
        1.0,
        0.0,
    );

    assert!(segments_match(
        &build_strike_segments(&effect, seed, reseed),
        &expected
    ));
}

fn growth_effect(growth_time: f32) -> LightningEffect {
    let mut effect = LightningEffect::default();
    effect.shape.source = LightningSource::Point;
    effect.shape.end_offset = [0.0, -8.0, 0.0];
    effect.timing.growth_time = growth_time;
    effect
}

fn segments_at_burst_time(effect: &LightningEffect, tau: f32) -> Vec<Segment> {
    build_lightning_segments(effect, timing::burst_start_time(effect, 0) + tau)
}

fn max_arrival(segments: &[Segment]) -> f32 {
    segments
        .iter()
        .map(|seg| seg.arrival_end)
        .fold(0.0f32, f32::max)
}

#[test]
fn test_growth_time_zero_is_unchanged() {
    let effect = growth_effect(0.0);
    let tau = effect.timing.attack_time + effect.timing.sustain_time * 0.5;

    let segments = segments_at_burst_time(&effect, tau);
    assert!(!segments.is_empty(), "should have segments");
    assert!(
        segments
            .iter()
            .any(|seg| length(difference(seg.b, effect.shape.end_offset)) < 1e-3),
        "growth_time=0 should reach the full end_offset"
    );
}

#[test]
fn test_growth_time_half_tau_clips_to_halfway() {
    let tau = 0.03;
    let effect = growth_effect(2.0 * tau);

    let half_arrival = 0.5 * max_arrival(&segments_at_burst_time(&growth_effect(0.0), tau));
    let segments = segments_at_burst_time(&effect, tau);
    assert!(!segments.is_empty(), "should have segments");

    for seg in &segments {
        assert!(
            seg.arrival_end <= half_arrival + 1e-4,
            "arrival {} > {half_arrival} + 1e-4",
            seg.arrival_end
        );
    }
    let farthest_arrival = max_arrival(&segments);
    assert!(
        (farthest_arrival - half_arrival).abs() < 1e-3,
        "the front should reach {half_arrival}, got {farthest_arrival}"
    );
}

#[test]
fn test_growth_time_tau_ge_growth_time_is_unchanged() {
    let tau = 0.03;
    let whole = segments_at_burst_time(&growth_effect(0.0), tau);
    assert!(!whole.is_empty(), "should have segments");

    for growth_time in [tau, 0.5 * tau] {
        let grown = segments_at_burst_time(&growth_effect(growth_time), tau);
        assert_eq!(
            grown, whole,
            "growth_time {growth_time} must not clip at tau {tau}"
        );
    }
}

#[test]
fn test_growth_time_shell_source_unchanged() {
    let tau = 0.03;
    let mut whole_effect = growth_effect(0.0);
    whole_effect.shape.source = LightningSource::Shell { radius: 1.0 };
    let mut grown_effect = whole_effect.clone();
    grown_effect.timing.growth_time = 2.0 * tau;

    assert_eq!(
        segments_at_burst_time(&grown_effect, tau),
        segments_at_burst_time(&whole_effect, tau),
        "shell sources ignore growth_time"
    );
}

#[test]
fn test_growth_reveals_the_interval_before_a_waypoint_first() {
    let tau = 0.03;
    let waypoint = [0.0, -10.0, 0.0];
    let mut whole_effect = waypoint_effect(&[waypoint]);
    whole_effect.shape.end_offset = [0.0, -4.0, 0.0];
    whole_effect.branch.count = 0.0;
    whole_effect.timing.growth_time = 0.0;

    let whole = segments_at_burst_time(&whole_effect, tau);
    let waypoint_index = whole
        .iter()
        .position(|seg| seg.b == waypoint)
        .expect("the main path passes through the waypoint");

    let mut crossed_the_waypoint = false;
    for front_fraction in [0.3, 0.6, 0.9] {
        let mut grown_effect = whole_effect.clone();
        grown_effect.timing.growth_time = tau / front_fraction;
        let grown = segments_at_burst_time(&grown_effect, tau);

        let revealed_after_waypoint = grown.len() > waypoint_index + 1;
        crossed_the_waypoint |= revealed_after_waypoint;
        if revealed_after_waypoint {
            assert_eq!(
                grown[..=waypoint_index],
                whole[..=waypoint_index],
                "front {front_fraction}: the interval before the waypoint must be complete"
            );
        } else {
            assert!(
                grown
                    .iter()
                    .all(|seg| seg.arrival_end <= whole[waypoint_index].arrival_end),
                "front {front_fraction}: only the interval before the waypoint may show"
            );
        }
    }
    assert!(crossed_the_waypoint, "the front must pass the waypoint");
}
