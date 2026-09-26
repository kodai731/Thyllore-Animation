use super::path::{branch_detail_levels, compute_branch_tangent, displace_path, emit_path};
use super::segment::{
    compute_distance, push_segment, segment_length, Segment, LIGHTNING_MAX_SEGMENTS,
};
use crate::LightningEffect;
use thyllore_math_core::hash_f32;

const END_ZONE_ANGLE_SCALE: f32 = 1.5;
const BRANCH_FADE_WIDTH: f32 = 0.05;

fn compute_branch_fade(effective_probability: f32, hash_value: f32) -> f32 {
    ((effective_probability - hash_value) / BRANCH_FADE_WIDTH).clamp(0.0, 1.0)
}

struct BranchSite {
    segment_index: usize,
    intensity_scale: f32,
}

fn compute_midpoint_progress(segments: &[Segment]) -> Vec<f32> {
    let lengths: Vec<f32> = segments.iter().map(segment_length).collect();
    let total_length: f32 = lengths.iter().sum();
    if total_length <= 0.0 {
        return Vec::new();
    }

    let mut travelled = 0.0f32;
    lengths
        .iter()
        .map(|length| {
            let midpoint = travelled + length * 0.5;
            travelled += length;
            midpoint / total_length
        })
        .collect()
}

fn pick_counted_branch_sites(
    segments: &[Segment],
    effect: &LightningEffect,
    seed: u32,
    reseed: u32,
) -> Vec<BranchSite> {
    let count = effect.branch.count;
    let zone_start = effect.branch.zone_start;
    let zone_end = effect.branch.zone_end;
    if count <= 0.0 || zone_start >= zone_end {
        return Vec::new();
    }

    let midpoint_progress = compute_midpoint_progress(segments);
    let slot_count = count.ceil() as u32;
    let partial_slot = count.floor() as u32;
    let slot_width = (zone_end - zone_start) / slot_count as f32;

    (0..slot_count)
        .filter_map(|slot| {
            let slot_start = zone_start + slot_width * slot as f32;
            let slot_end = zone_start + slot_width * (slot + 1) as f32;
            let target = slot_start + hash_f32(&[seed, reseed, slot, 0, 7]) * slot_width;
            let segment_index = midpoint_progress
                .iter()
                .enumerate()
                .filter(|(_, progress)| slot_start <= **progress && **progress < slot_end)
                .min_by(|(_, a), (_, b)| (**a - target).abs().total_cmp(&(**b - target).abs()))
                .map(|(index, _)| index)?;

            let intensity_scale = if slot == partial_slot {
                count.fract()
            } else {
                1.0
            };
            Some(BranchSite {
                segment_index,
                intensity_scale,
            })
        })
        .collect()
}

fn pick_probable_branch_sites(
    segment_count: usize,
    effect: &LightningEffect,
    seed: u32,
    reseed: u32,
    depth: u32,
) -> Vec<BranchSite> {
    (0..segment_count)
        .filter_map(|i| {
            let hash_value = hash_f32(&[seed, reseed, i as u32, depth]);
            (hash_value < effect.branch.probability).then(|| BranchSite {
                segment_index: i,
                intensity_scale: compute_branch_fade(effect.branch.probability, hash_value),
            })
        })
        .collect()
}

struct BranchChord {
    displacement_stream: u32,
    start: [f32; 3],
    end: [f32; 3],
    intensity: f32,
    arrival: f32,
}

struct GrownBranch {
    segments: Vec<Segment>,
    levels: u32,
}

fn compute_chord_length(chord: &BranchChord) -> f32 {
    compute_distance(chord.start, chord.end)
}

fn plan_branch_chords(
    parent: &[Segment],
    sites: &[BranchSite],
    tangent_targets: &[[f32; 3]],
    effect: &LightningEffect,
    seed: u32,
    reseed: u32,
    depth: u32,
) -> Vec<BranchChord> {
    let parent_count = parent.len();
    let mut remaining_len_after = vec![0.0f32; parent_count + 1];
    for i in (0..parent_count).rev() {
        remaining_len_after[i] = remaining_len_after[i + 1] + segment_length(&parent[i]);
    }

    sites
        .iter()
        .map(|site| {
            let i = site.segment_index;
            let seg = &parent[i];
            let progress = i as f32 / parent_count as f32;
            let is_end_zone = progress >= 0.9;

            let mid_pos: [f32; 3] = [
                (seg.a[0] + seg.b[0]) * 0.5,
                (seg.a[1] + seg.b[1]) * 0.5,
                (seg.a[2] + seg.b[2]) * 0.5,
            ];

            let chord_remaining_len = segment_length(seg) * 0.5 + remaining_len_after[i + 1];

            let end_pos = tangent_targets[i];
            let parent_tangent: [f32; 3] = [
                end_pos[0] - mid_pos[0],
                end_pos[1] - mid_pos[1],
                end_pos[2] - mid_pos[2],
            ];

            let azimuth_hash = hash_f32(&[seed, reseed, i as u32, depth, 0]);
            let angle_hash = hash_f32(&[seed, reseed, i as u32, depth, 3]);

            let branch_angle = if is_end_zone {
                effect.branch.angle * END_ZONE_ANGLE_SCALE
            } else {
                effect.branch.angle
            };
            let child_tangent =
                compute_branch_tangent(parent_tangent, branch_angle, azimuth_hash, angle_hash);
            let child_length = effect.branch.length_ratio * chord_remaining_len;

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

            BranchChord {
                displacement_stream: i as u32,
                start: mid_pos,
                end: child_end,
                intensity: seg.intensity * effect.branch.intensity_ratio * site.intensity_scale,
                arrival: seg.arrival_start + segment_length(seg) * 0.5,
            }
        })
        .collect()
}

fn grow_branch(
    chord: &BranchChord,
    effect: &LightningEffect,
    seed: u32,
    reseed: u32,
    levels: u32,
) -> GrownBranch {
    let points = displace_path(
        effect,
        seed,
        reseed,
        chord.displacement_stream,
        chord.start,
        chord.end,
        levels,
    );
    let radius_start = effect.shape.core_radius * effect.branch.radius_ratio;
    let radius_end = radius_start * effect.shape.tip_radius_ratio;

    let mut segments = Vec::new();
    emit_path(
        &mut segments,
        &points,
        radius_start,
        radius_end,
        chord.intensity,
        chord.arrival,
    );
    GrownBranch { segments, levels }
}

fn spawn_root_branches(
    main_path: &[Segment],
    interval_ends: &[[f32; 3]],
    effect: &LightningEffect,
    seed: u32,
    reseed: u32,
) -> Vec<GrownBranch> {
    let sites = pick_counted_branch_sites(main_path, effect, seed, reseed);
    let chords = plan_branch_chords(main_path, &sites, interval_ends, effect, seed, reseed, 0);

    let remaining_segments = LIGHTNING_MAX_SEGMENTS.saturating_sub(main_path.len());
    let root_budget = if effect.branch.depth > 0 {
        remaining_segments * 2 / 3
    } else {
        remaining_segments
    };
    let total_length: f32 = chords.iter().map(compute_chord_length).sum();
    let max_levels = effect.shape.detail_levels.saturating_sub(1);

    chords
        .iter()
        .map(|chord| {
            let levels = branch_detail_levels(
                root_budget,
                compute_chord_length(chord),
                total_length,
                max_levels,
            );
            grow_branch(chord, effect, seed, reseed, levels)
        })
        .collect()
}

fn spawn_child_branches(
    parent: &GrownBranch,
    effect: &LightningEffect,
    seed: u32,
    reseed: u32,
    depth: u32,
) -> Vec<GrownBranch> {
    let sites = pick_probable_branch_sites(parent.segments.len(), effect, seed, reseed, depth);
    let levels = parent.levels.saturating_sub(1).max(1);
    let parent_end = parent
        .segments
        .last()
        .map(|seg| vec![seg.b; parent.segments.len()])
        .unwrap_or_default();

    plan_branch_chords(
        &parent.segments,
        &sites,
        &parent_end,
        effect,
        seed,
        reseed,
        depth,
    )
    .iter()
    .map(|chord| grow_branch(chord, effect, seed, reseed, levels))
    .collect()
}

fn push_branches(segments: &mut Vec<Segment>, branches: &[GrownBranch]) {
    for seg in branches.iter().flat_map(|branch| &branch.segments) {
        if !push_segment(segments, *seg) {
            return;
        }
    }
}

pub(super) fn spawn_branches(
    segments: &mut Vec<Segment>,
    interval_ends: &[[f32; 3]],
    effect: &LightningEffect,
    seed: u32,
    reseed: u32,
) {
    let mut generation = spawn_root_branches(segments, interval_ends, effect, seed, reseed);

    for depth in 1..=effect.branch.depth {
        push_branches(segments, &generation);
        if segments.len() >= LIGHTNING_MAX_SEGMENTS {
            return;
        }

        generation = generation
            .iter()
            .flat_map(|parent| spawn_child_branches(parent, effect, seed, reseed, depth))
            .collect();
    }

    push_branches(segments, &generation);
}

#[cfg(test)]
mod tests {
    use super::super::build_strike_segments;
    use super::super::test_support::{
        build_root_branch_paths, difference, dot, length, straight_counted_branch_effect, WAYPOINTS,
    };
    use super::*;
    use thyllore_math_core::hash_u32;

    fn branch_chord_lengths(branch_segments: &[Segment]) -> Vec<f32> {
        let mut chords = Vec::new();
        let mut path_start = match branch_segments.first() {
            Some(seg) => seg.a,
            None => return chords,
        };

        for (i, seg) in branch_segments.iter().enumerate() {
            let previous_end = branch_segments[i.saturating_sub(1)].b;
            if i > 0 && length(difference(seg.a, previous_end)) > 1e-6 {
                chords.push(length(difference(previous_end, path_start)));
                path_start = seg.a;
            }
        }
        let last_end = branch_segments[branch_segments.len() - 1].b;
        chords.push(length(difference(last_end, path_start)));

        chords
    }

    #[test]
    fn test_branch_length_follows_parent_remaining_length() {
        let mut effect = LightningEffect::default();
        effect.shape.detail_levels = 4;
        effect.branch.length_ratio = 0.5;
        effect.branch.depth = 0;
        effect.branch.count = 0.0;

        let seed = hash_u32(&[0, 0]);
        let main_path = build_strike_segments(&effect, seed, 0);

        effect.branch.count = 4.0;
        let with_branches = build_strike_segments(&effect, seed, 0);

        assert!(
            with_branches.len() > main_path.len(),
            "branch_count=4.0 must add branch segments, got {} vs {}",
            with_branches.len(),
            main_path.len()
        );

        let main_path_len: f32 = main_path.iter().map(segment_length).sum();
        let longest_branch_chord = branch_chord_lengths(&with_branches[main_path.len()..])
            .into_iter()
            .fold(0.0f32, f32::max);

        assert!(
            longest_branch_chord > main_path_len * 0.05,
            "longest branch chord {longest_branch_chord} must exceed 5% of parent path length {main_path_len}"
        );
    }

    #[test]
    fn test_compute_branch_fade_at_threshold() {
        let fade = compute_branch_fade(0.5, 0.5);
        assert!(
            (fade - 0.0).abs() < 1e-6,
            "expected 0.0 at threshold, got {fade}"
        );
    }

    #[test]
    fn test_compute_branch_fade_far_from_threshold() {
        let fade = compute_branch_fade(0.5, 0.4);
        assert!(
            (fade - 1.0).abs() < 1e-6,
            "expected 1.0 far from threshold, got {fade}"
        );
    }

    #[test]
    fn test_compute_branch_fade_between() {
        let fade = compute_branch_fade(0.5, 0.48);
        assert!(
            (fade - 0.4).abs() < 1e-6,
            "expected 0.4 between threshold and full, got {fade}"
        );
    }

    #[test]
    fn test_compute_branch_fade_monotonicity() {
        let fades: Vec<f32> = (0..=10)
            .map(|step| compute_branch_fade(0.5, 0.5 - step as f32 * 0.005))
            .collect();

        for pair in fades.windows(2) {
            assert!(pair[1] > pair[0], "fade not increasing: {fades:?}");
        }
    }

    #[test]
    fn test_integer_branch_count_spawns_exactly_that_many_root_branches() {
        let effect = straight_counted_branch_effect(4.0);
        assert_eq!(build_root_branch_paths(&effect).len(), 4);
    }

    #[test]
    fn test_waypoints_keep_the_root_branch_count() {
        for count in 1..=3 {
            let mut effect = straight_counted_branch_effect(4.0);
            effect.shape.waypoints[..count].copy_from_slice(&WAYPOINTS[..count]);
            effect.shape.waypoint_count = count as u32;
            assert_eq!(
                build_root_branch_paths(&effect).len(),
                4,
                "{count} waypoints"
            );
        }
    }

    #[test]
    fn test_bolt_preset_spawns_every_counted_root_branch() {
        let mut effect = LightningEffect::default();
        crate::lightning::apply_lightning_preset(&mut effect, "bolt");
        effect.timing.burst_jitter = 0.0;
        effect.branch.depth = 0;

        for branch_count in [16.0, 24.0] {
            effect.branch.count = branch_count;
            assert_eq!(
                build_root_branch_paths(&effect).len(),
                branch_count as usize,
                "branch_count={branch_count}"
            );
        }
    }

    #[test]
    fn test_root_branches_start_inside_the_branch_zone() {
        let mut effect = straight_counted_branch_effect(8.0);
        effect.branch.zone_start = 0.3;
        effect.branch.zone_end = 0.7;

        let paths = build_root_branch_paths(&effect);
        assert!(!paths.is_empty(), "the zone must still hold branches");

        let chord = effect.shape.end_offset;
        for path in &paths {
            let progress = dot(path[0].a, chord) / dot(chord, chord);
            assert!(
                (0.3..0.7).contains(&progress),
                "branch starts at progress {progress}, outside [0.3, 0.7)"
            );
        }
    }

    #[test]
    fn test_fractional_branch_count_scales_the_last_root_branch() {
        let effect = straight_counted_branch_effect(2.5);
        let paths = build_root_branch_paths(&effect);
        assert_eq!(paths.len(), 3);

        let full = effect.branch.intensity_ratio;
        let expected = [full, full, full * 0.5];
        for (path, expected_intensity) in paths.iter().zip(expected) {
            for seg in path {
                assert!(
                    (seg.intensity - expected_intensity).abs() < 1e-6,
                    "intensity {} expected {expected_intensity}",
                    seg.intensity
                );
            }
        }
    }
}
