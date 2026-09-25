use crate::lightning::analytic::hash::{hash_f32, HashChannel};
use crate::lightning::analytic::timing;
use crate::lightning::effect::LightningSource;
use crate::LightningEffect;
use cgmath::{InnerSpace, Quaternion, Rad, Rotation, Rotation3, Vector3};

pub const LIGHTNING_MAX_SEGMENTS: usize = 256;
const END_ZONE_ANGLE_SCALE: f32 = 1.5;
const BRANCH_FADE_WIDTH: f32 = 0.05;

fn push_segment(segments: &mut Vec<Segment>, seg: Segment) -> bool {
    if segments.len() >= LIGHTNING_MAX_SEGMENTS {
        return false;
    }
    segments.push(seg);
    true
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub r0: f32,
    pub r1: f32,
    /// Relative envelope (branch dimming x stroke x flicker); the shader applies
    /// `core_intensity` / `rim_intensity` on top, so this must stay O(1).
    pub intensity: f32,
}

fn segment_progress(p: [f32; 3], start: [f32; 3], dir: [f32; 3], dir_sq_len: f32) -> f32 {
    (dir[0] * (p[0] - start[0]) + dir[1] * (p[1] - start[1]) + dir[2] * (p[2] - start[2]))
        / dir_sq_len
}

fn interpolate(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn clip_to_growth_front(
    segments: &[Segment],
    start: [f32; 3],
    end: [f32; 3],
    front: f32,
) -> Vec<Segment> {
    let dir = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
    let dir_sq_len = dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2];

    if dir_sq_len < 1e-12 {
        return segments.to_vec();
    }

    let mut out = Vec::with_capacity(segments.len());
    for seg in segments {
        let s_a = segment_progress(seg.a, start, dir, dir_sq_len);
        let s_b = segment_progress(seg.b, start, dir, dir_sq_len);

        if s_a <= front && s_b <= front {
            out.push(*seg);
        } else if s_a > front && s_b <= front {
            let frac = (front - s_b) / (s_a - s_b);
            let new_a = [
                interpolate(seg.b[0], seg.a[0], frac),
                interpolate(seg.b[1], seg.a[1], frac),
                interpolate(seg.b[2], seg.a[2], frac),
            ];
            let new_r0 = interpolate(seg.r1, seg.r0, frac);
            out.push(Segment {
                a: new_a,
                b: seg.b,
                r0: new_r0,
                r1: seg.r1,
                intensity: seg.intensity,
            });
        } else if s_a <= front {
            let frac = (front - s_a) / (s_b - s_a);
            let new_b = [
                interpolate(seg.a[0], seg.b[0], frac),
                interpolate(seg.a[1], seg.b[1], frac),
                interpolate(seg.a[2], seg.b[2], frac),
            ];
            let new_r1 = interpolate(seg.r0, seg.r1, frac);
            out.push(Segment {
                a: seg.a,
                b: new_b,
                r0: seg.r0,
                r1: new_r1,
                intensity: seg.intensity,
            });
        }
    }
    out
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

    if effect.end_variance > 0.0 {
        let azimuth_hash = hash_f32(&[effect.seed, k, HashChannel::EndVariance.into(), 0]);
        let radius_hash = hash_f32(&[effect.seed, k, HashChannel::EndVariance.into(), 1]);
        charged.end_offset = scatter_end_offset(
            charged.end_offset,
            effect.end_variance,
            azimuth_hash,
            radius_hash,
        );
    }

    let mut segments = build_strike_segments(&charged, burst_seed, reseed);

    if let LightningSource::Point = effect.source {
        if effect.growth_time > 0.0 {
            let front = (tau / effect.growth_time).min(1.0);
            segments = clip_to_growth_front(&segments, [0.0, 0.0, 0.0], charged.end_offset, front);
        }
    }

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

    let canonical_down = -Vector3::unit_y();
    let u = if axis.dot(canonical_down) < -1.0 + 1e-6 {
        Vector3::unit_x()
    } else {
        Quaternion::from_arc(canonical_down, axis, None).rotate_vector(Vector3::unit_x())
    };

    let v = axis.cross(u);
    (u.into(), v.into())
}

fn scatter_end_offset(
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

fn rotate_around_axis(v: [f32; 3], axis: [f32; 3], angle: f32) -> [f32; 3] {
    let axis = Vector3::from(axis);
    if axis.magnitude2() <= 0.0 {
        return v;
    }

    let rotation = Quaternion::from_axis_angle(axis.normalize(), Rad(angle));
    rotation.rotate_vector(Vector3::from(v)).into()
}

fn compute_branch_tangent(
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

fn compute_branch_fade(effective_probability: f32, hash_value: f32) -> f32 {
    ((effective_probability - hash_value) / BRANCH_FADE_WIDTH).clamp(0.0, 1.0)
}

fn branch_detail_levels(budget: usize, length: f32, total_length: f32, max_levels: u32) -> u32 {
    let segment_share = budget as f32 * length / total_length;
    let levels = segment_share.log2().floor().max(1.0) as u32;
    levels.min(max_levels)
}

pub(crate) fn segment_length(seg: &Segment) -> f32 {
    let d = [
        seg.b[0] - seg.a[0],
        seg.b[1] - seg.a[1],
        seg.b[2] - seg.a[2],
    ];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
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
    let count = effect.branch_count;
    let zone_start = effect.branch_zone_start;
    let zone_end = effect.branch_zone_end;
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
            (hash_value < effect.branch_probability).then(|| BranchSite {
                segment_index: i,
                intensity_scale: compute_branch_fade(effect.branch_probability, hash_value),
            })
        })
        .collect()
}

struct BranchChord {
    displacement_stream: u32,
    start: [f32; 3],
    end: [f32; 3],
    intensity: f32,
}

struct GrownBranch {
    segments: Vec<Segment>,
    levels: u32,
}

fn compute_chord_length(chord: &BranchChord) -> f32 {
    let d = [
        chord.end[0] - chord.start[0],
        chord.end[1] - chord.start[1],
        chord.end[2] - chord.start[2],
    ];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

fn plan_branch_chords(
    parent: &[Segment],
    sites: &[BranchSite],
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

            let end_pos = parent[parent_count - 1].b;
            let parent_tangent: [f32; 3] = [
                end_pos[0] - mid_pos[0],
                end_pos[1] - mid_pos[1],
                end_pos[2] - mid_pos[2],
            ];

            let azimuth_hash = hash_f32(&[seed, reseed, i as u32, depth, 0]);
            let angle_hash = hash_f32(&[seed, reseed, i as u32, depth, 3]);

            let branch_angle = if is_end_zone {
                effect.branch_angle * END_ZONE_ANGLE_SCALE
            } else {
                effect.branch_angle
            };
            let child_tangent =
                compute_branch_tangent(parent_tangent, branch_angle, azimuth_hash, angle_hash);
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

            BranchChord {
                displacement_stream: i as u32,
                start: mid_pos,
                end: child_end,
                intensity: seg.intensity * effect.branch_intensity_ratio * site.intensity_scale,
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
    let radius_start = effect.core_radius * effect.branch_radius_ratio;
    let radius_end = radius_start * effect.tip_radius_ratio;

    let mut segments = Vec::new();
    emit_path(
        &mut segments,
        &points,
        radius_start,
        radius_end,
        chord.intensity,
    );
    GrownBranch { segments, levels }
}

fn spawn_root_branches(
    main_path: &[Segment],
    effect: &LightningEffect,
    seed: u32,
    reseed: u32,
) -> Vec<GrownBranch> {
    let sites = pick_counted_branch_sites(main_path, effect, seed, reseed);
    let chords = plan_branch_chords(main_path, &sites, effect, seed, reseed, 0);

    let remaining_segments = LIGHTNING_MAX_SEGMENTS.saturating_sub(main_path.len());
    let root_budget = if effect.branch_depth > 0 {
        remaining_segments * 2 / 3
    } else {
        remaining_segments
    };
    let total_length: f32 = chords.iter().map(compute_chord_length).sum();
    let max_levels = effect.detail_levels.saturating_sub(1);

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

    plan_branch_chords(&parent.segments, &sites, effect, seed, reseed, depth)
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

fn spawn_branches(segments: &mut Vec<Segment>, effect: &LightningEffect, seed: u32, reseed: u32) {
    let mut generation = spawn_root_branches(segments, effect, seed, reseed);

    for depth in 1..=effect.branch_depth {
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
                intensity: 1.0,
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
            let child_intensity = effect.branch_intensity_ratio;

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

            let points = displace_path(
                effect,
                seed,
                reseed,
                s,
                start,
                end_offset,
                effect.detail_levels,
            );

            let mut strike_segments = Vec::new();
            emit_path(
                &mut strike_segments,
                &points,
                effect.core_radius,
                effect.core_radius * effect.tip_radius_ratio,
                1.0,
            );
            spawn_branches(&mut strike_segments, effect, seed, reseed);

            for seg in strike_segments {
                if !push_segment(&mut segments, seg) {
                    break;
                }
            }

            if segments.len() >= LIGHTNING_MAX_SEGMENTS {
                break;
            }
        }
    }

    segments
}

/// Corners of the local axis-aligned box enclosing every rim capsule alive at `t`.
pub fn compute_lightning_segment_aabb(
    effect: &LightningEffect,
    t: f32,
) -> Option<[Vector3<f32>; 8]> {
    let mut bounds: Option<(Vector3<f32>, Vector3<f32>)> = None;
    for seg in build_lightning_segments(effect, t) {
        let rim_radius = seg.r0.max(seg.r1) * effect.rim_ratio;
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
        effect.branch_count = 0.0;

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
        effect.branch_count = 0.0;
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

    fn straight_point_effect(end_variance: f32) -> LightningEffect {
        let mut effect = LightningEffect::default();
        effect.source = LightningSource::Point;
        effect.end_offset = [0.0, -6.0, 0.0];
        effect.detail_levels = 0;
        effect.branch_count = 0.0;
        effect.burst_count = 2;
        effect.end_variance = end_variance;
        effect
    }

    fn strike_end_at(
        effect: &LightningEffect,
        burst_index: u32,
        sustain_fraction: f32,
    ) -> [f32; 3] {
        let tau = effect.attack_time + effect.sustain_time * sustain_fraction;
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
        assert!(length(difference(end, effect.end_offset)) < 1e-5, "{end:?}");
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

    fn difference(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
    }

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
        effect.detail_levels = 4;
        effect.branch_length_ratio = 0.5;
        effect.branch_depth = 0;
        effect.branch_count = 0.0;

        let seed = crate::lightning::hash_u32(&[0, 0]);
        let main_path = build_strike_segments(&effect, seed, 0);

        effect.branch_count = 4.0;
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

    fn time_inside_first_burst(effect: &LightningEffect) -> f32 {
        timing::burst_start_time(effect, 0) + effect.attack_time + effect.sustain_time * 0.5
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

    fn expanded_segment_bounds(
        effect: &LightningEffect,
        segments: &[Segment],
    ) -> ([f32; 3], [f32; 3]) {
        let rim_radius = |seg: &Segment| seg.r0.max(seg.r1) * effect.rim_ratio;
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
        effect.detail_levels = 0;
        effect.branch_count = 0.0;
        effect.end_offset = [3.0, -8.0, 1.0];
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
        effect.branch_count = 8.0;
        effect.branch_depth = 2;
        let t = time_inside_first_burst(&effect);

        let mut unbranched = effect.clone();
        unbranched.branch_count = 0.0;
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
    fn test_bolt_preset_branches_dont_grow_upwards() {
        let mut effect = LightningEffect::default();
        crate::lightning::apply_lightning_preset(&mut effect, "bolt");
        effect.burst_jitter = 0.0;

        let segments = build_lightning_segments(&effect, 0.3);
        assert!(
            !segments.is_empty(),
            "bolt preset at t=0.3 must produce segments"
        );

        let max_y_above_origin = 0.15
            * (effect.end_offset[0] * effect.end_offset[0]
                + effect.end_offset[1] * effect.end_offset[1]
                + effect.end_offset[2] * effect.end_offset[2])
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

    fn straight_counted_branch_effect(branch_count: f32) -> LightningEffect {
        let mut effect = LightningEffect::default();
        effect.detail_levels = 4;
        effect.tortuosity = 0.0;
        effect.branch_depth = 0;
        effect.branch_count = branch_count;
        effect.end_offset = [3.0, -8.0, 1.0];
        effect
    }

    fn build_root_branch_paths(effect: &LightningEffect) -> Vec<Vec<Segment>> {
        let seed = crate::lightning::hash_u32(&[0, 0]);
        let segments = build_strike_segments(effect, seed, 0);
        let branch_segments = &segments[1 << effect.detail_levels..];

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

    #[test]
    fn test_integer_branch_count_spawns_exactly_that_many_root_branches() {
        let effect = straight_counted_branch_effect(4.0);
        assert_eq!(build_root_branch_paths(&effect).len(), 4);
    }

    #[test]
    fn test_bolt_preset_spawns_every_counted_root_branch() {
        let mut effect = LightningEffect::default();
        crate::lightning::apply_lightning_preset(&mut effect, "bolt");
        effect.burst_jitter = 0.0;
        effect.branch_depth = 0;

        for branch_count in [16.0, 24.0] {
            effect.branch_count = branch_count;
            assert_eq!(
                build_root_branch_paths(&effect).len(),
                branch_count as usize,
                "branch_count={branch_count}"
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

    #[test]
    fn test_root_branches_start_inside_the_branch_zone() {
        let mut effect = straight_counted_branch_effect(8.0);
        effect.branch_zone_start = 0.3;
        effect.branch_zone_end = 0.7;

        let paths = build_root_branch_paths(&effect);
        assert!(!paths.is_empty(), "the zone must still hold branches");

        let chord = effect.end_offset;
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

        let full = effect.branch_intensity_ratio;
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

    fn growth_effect(growth_time: f32) -> LightningEffect {
        let mut effect = LightningEffect::default();
        effect.source = LightningSource::Point;
        effect.end_offset = [0.0, -8.0, 0.0];
        effect.growth_time = growth_time;
        effect
    }

    fn segments_at_burst_time(effect: &LightningEffect, tau: f32) -> Vec<Segment> {
        build_lightning_segments(effect, timing::burst_start_time(effect, 0) + tau)
    }

    fn growth_progress(p: [f32; 3], end: [f32; 3]) -> f32 {
        dot(p, end) / dot(end, end)
    }

    #[test]
    fn test_growth_time_zero_is_unchanged() {
        let effect = growth_effect(0.0);
        let tau = effect.attack_time + effect.sustain_time * 0.5;

        let segments = segments_at_burst_time(&effect, tau);
        assert!(!segments.is_empty(), "should have segments");
        assert!(
            segments
                .iter()
                .any(|seg| length(difference(seg.b, effect.end_offset)) < 1e-3),
            "growth_time=0 should reach the full end_offset"
        );
    }

    #[test]
    fn test_growth_time_half_tau_clips_to_halfway() {
        let tau = 0.03;
        let effect = growth_effect(2.0 * tau);

        let segments = segments_at_burst_time(&effect, tau);
        assert!(!segments.is_empty(), "should have segments");

        let mut farthest_progress = 0.0f32;
        for seg in &segments {
            let s_a = growth_progress(seg.a, effect.end_offset);
            let s_b = growth_progress(seg.b, effect.end_offset);
            assert!(s_a <= 0.5 + 1e-4, "segment a progress {s_a} > 0.5 + 1e-4");
            assert!(s_b <= 0.5 + 1e-4, "segment b progress {s_b} > 0.5 + 1e-4");
            farthest_progress = farthest_progress.max(s_b);
        }
        assert!(
            (farthest_progress - 0.5).abs() < 1e-3,
            "main path should reach s≈0.5, got {farthest_progress}"
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
    fn test_clip_to_growth_front_spanning_segment_interpolation() {
        let segments = vec![
            Segment {
                a: [0.0, 0.0, 0.0],
                b: [0.0, -4.0, 0.0],
                r0: 1.0,
                r1: 0.5,
                intensity: 1.0,
            },
            Segment {
                a: [0.0, -4.0, 0.0],
                b: [0.0, -8.0, 0.0],
                r0: 0.5,
                r1: 0.3,
                intensity: 1.0,
            },
        ];

        let clipped = clip_to_growth_front(&segments, [0.0, 0.0, 0.0], [0.0, -8.0, 0.0], 0.25);

        assert_eq!(clipped.len(), 1, "the segment past the front is discarded");
        let seg = &clipped[0];
        assert_eq!(seg.a, [0.0, 0.0, 0.0]);
        assert!(
            length(difference(seg.b, [0.0, -2.0, 0.0])) < 1e-6,
            "b = {:?}",
            seg.b
        );
        assert!((seg.r0 - 1.0).abs() < 1e-6);
        assert!((seg.r1 - 0.75).abs() < 1e-6, "r1 = {}", seg.r1);
    }

    #[test]
    fn test_clip_to_growth_front_all_before_front() {
        let segments = vec![Segment {
            a: [0.0, 0.0, 0.0],
            b: [0.0, -4.0, 0.0],
            r0: 1.0,
            r1: 0.5,
            intensity: 1.0,
        }];

        let clipped = clip_to_growth_front(&segments, [0.0, 0.0, 0.0], [0.0, -8.0, 0.0], 0.7);
        assert_eq!(clipped.len(), 1);
        assert_eq!(clipped[0].b, [0.0, -4.0, 0.0]);
    }

    #[test]
    fn test_clip_to_growth_front_all_after_front() {
        let segments = vec![Segment {
            a: [0.0, -4.0, 0.0],
            b: [0.0, -8.0, 0.0],
            r0: 0.5,
            r1: 0.3,
            intensity: 1.0,
        }];

        let clipped = clip_to_growth_front(&segments, [0.0, 0.0, 0.0], [0.0, -8.0, 0.0], 0.3);
        assert!(
            clipped.is_empty(),
            "all segments after front should be discarded"
        );
    }

    #[test]
    fn test_growth_time_shell_source_unchanged() {
        let tau = 0.03;
        let mut whole_effect = growth_effect(0.0);
        whole_effect.source = LightningSource::Shell { radius: 1.0 };
        let mut grown_effect = whole_effect.clone();
        grown_effect.growth_time = 2.0 * tau;

        assert_eq!(
            segments_at_burst_time(&grown_effect, tau),
            segments_at_burst_time(&whole_effect, tau),
            "shell sources ignore growth_time"
        );
    }

    #[test]
    fn test_clip_to_growth_front_backward_segment() {
        let segments = vec![Segment {
            a: [0.0, -6.0, 0.0],
            b: [0.0, -2.0, 0.0],
            r0: 0.3,
            r1: 0.8,
            intensity: 1.0,
        }];

        let clipped = clip_to_growth_front(&segments, [0.0, 0.0, 0.0], [0.0, -8.0, 0.0], 0.5);

        assert_eq!(clipped.len(), 1, "the backward segment is kept");
        let seg = &clipped[0];
        assert!(
            length(difference(seg.a, [0.0, -4.0, 0.0])) < 1e-6,
            "a = {:?}",
            seg.a
        );
        assert_eq!(seg.b, [0.0, -2.0, 0.0]);
        assert!((seg.r0 - 0.55).abs() < 1e-6, "r0 = {}", seg.r0);
        assert!((seg.r1 - 0.8).abs() < 1e-6);
    }
}
