use std::borrow::Cow;
use std::collections::BTreeSet;
use std::f32::consts::{PI, TAU};

use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::world_y_axis;

use thyllore_scene_core::find_scalar_param;

use crate::garment::components::chain_gen::{
    joint_name, ChainGenInput, ChainGenParams, ChainGenSettings, ChainRole, GeneratedChain,
    GeneratedJoint, InfluenceTarget, VertexInfluences, CHAIN_GEN_SCALAR_PARAMS,
};

const AZIMUTH_BIN_COUNT: usize = 36;
const KMEANS_ITERATION_COUNT: usize = 16;
const MAX_INFLUENCES_PER_VERTEX: usize = 4;
const ROOT_BAND_SLICE_COUNT: usize = 64;
const MIN_STRAND_AZIMUTH_CONCENTRATION: f32 = 0.5;

#[derive(Debug, Clone, Copy)]
struct VertexFrame {
    vertex: u32,
    azimuth: f32,
    progress: f32,
}

#[derive(Debug, Clone, Copy)]
struct AzimuthSample {
    azimuth: f32,
    weight: f32,
}

/// The hair root band in root-relative progress (distance down the up axis from the root bone).
#[derive(Debug, Clone, Copy)]
pub struct RootBand {
    pub equator_progress: f32,
    pub progress: f32,
    pub center: Vector3<f32>,
}

fn collect_target_vertices(input: &ChainGenInput) -> Vec<u32> {
    let vertex_count = input.positions.len() as u32;
    input
        .triangles
        .iter()
        .flatten()
        .copied()
        .filter(|&vertex| vertex < vertex_count)
        .collect::<BTreeSet<u32>>()
        .into_iter()
        .collect()
}

fn build_azimuth_basis(up: Vector3<f32>) -> (Vector3<f32>, Vector3<f32>) {
    let seed = if up.x.abs() < 0.9 {
        Vector3::unit_x()
    } else {
        Vector3::unit_z()
    };
    let reference = (seed - up * seed.dot(up)).normalize();
    (reference, up.cross(reference))
}

pub(crate) fn progress_of(
    position: Vector3<f32>,
    root_position: Vector3<f32>,
    up: Vector3<f32>,
) -> f32 {
    -(position - root_position).dot(up)
}

fn horizontal_radius(offset: Vector3<f32>, up: Vector3<f32>) -> f32 {
    (offset - up * offset.dot(up)).magnitude()
}

fn mean_position(positions: &[Vector3<f32>]) -> Vector3<f32> {
    positions
        .iter()
        .fold(Vector3::new(0.0, 0.0, 0.0), |sum, p| sum + p)
        / positions.len().max(1) as f32
}

/// The widest horizontal cross-section of the attachment surface, shifted by the setting.
fn estimate_root_band(
    attachment: &[Vector3<f32>],
    root_position: Vector3<f32>,
    offset: f32,
) -> RootBand {
    let up = world_y_axis();
    let center = mean_position(attachment);
    let progresses: Vec<f32> = attachment
        .iter()
        .map(|&p| progress_of(p, root_position, up))
        .collect();
    let min_progress = progresses.iter().copied().reduce(f32::min).unwrap_or(0.0);
    let max_progress = progresses.iter().copied().reduce(f32::max).unwrap_or(0.0);
    let slice_width = (max_progress - min_progress) / ROOT_BAND_SLICE_COUNT as f32;

    let slice_of = |progress: f32| -> usize {
        if slice_width > 0.0 {
            (((progress - min_progress) / slice_width) as usize).min(ROOT_BAND_SLICE_COUNT - 1)
        } else {
            0
        }
    };
    let mut widest_radius = vec![0.0f32; ROOT_BAND_SLICE_COUNT];
    for (&p, &progress) in attachment.iter().zip(&progresses) {
        let slice = slice_of(progress);
        widest_radius[slice] = widest_radius[slice].max(horizontal_radius(p - center, up));
    }
    let widest_slice = (0..ROOT_BAND_SLICE_COUNT)
        .max_by(|a, b| widest_radius[*a].total_cmp(&widest_radius[*b]))
        .unwrap_or(0);

    let slice_members: Vec<Vector3<f32>> = attachment
        .iter()
        .zip(&progresses)
        .filter(|(_, &progress)| slice_of(progress) == widest_slice)
        .map(|(&p, _)| p)
        .collect();

    let equator_progress = min_progress + (widest_slice as f32 + 0.5) * slice_width;
    RootBand {
        equator_progress,
        progress: equator_progress + offset,
        center: mean_position(&slice_members),
    }
}

/// The root band a hair generation uses; a skirt has none.
pub fn hair_root_band(input: &ChainGenInput, params: &ChainGenParams) -> Option<RootBand> {
    match params.role {
        ChainRole::Hair => {
            let vertices = collect_target_vertices(input);
            Some(estimate_root_band(
                &attachment_or_target(input, &vertices),
                input.root_position,
                params.settings.root_band_offset,
            ))
        }
        ChainRole::Skirt => None,
    }
}

fn compute_vertex_frames(
    input: &ChainGenInput,
    vertices: &[u32],
    azimuth_center: Vector3<f32>,
) -> Vec<VertexFrame> {
    let up = world_y_axis();
    let (reference, side) = build_azimuth_basis(up);

    vertices
        .iter()
        .map(|&vertex| {
            let position = input.positions[vertex as usize];
            let offset = position - azimuth_center;
            VertexFrame {
                vertex,
                azimuth: offset.dot(side).atan2(offset.dot(reference)),
                progress: progress_of(position, input.root_position, up),
            }
        })
        .collect()
}

fn azimuth_bin(azimuth: f32, bin_count: usize) -> usize {
    let normalized = azimuth.rem_euclid(TAU) / TAU;
    ((normalized * bin_count as f32) as usize).min(bin_count - 1)
}

fn smoothed_azimuth_histogram(samples: &[AzimuthSample]) -> Vec<f32> {
    let mut histogram = [0.0f32; AZIMUTH_BIN_COUNT];
    for sample in samples {
        histogram[azimuth_bin(sample.azimuth, AZIMUTH_BIN_COUNT)] += sample.weight;
    }
    (0..AZIMUTH_BIN_COUNT)
        .map(|bin| {
            let left = histogram[(bin + AZIMUTH_BIN_COUNT - 1) % AZIMUTH_BIN_COUNT];
            let right = histogram[(bin + 1) % AZIMUTH_BIN_COUNT];
            (left + histogram[bin] + right) / 3.0
        })
        .collect()
}

fn count_azimuth_modes(samples: &[AzimuthSample]) -> usize {
    let smoothed = smoothed_azimuth_histogram(samples);
    let mode_count = (0..AZIMUTH_BIN_COUNT)
        .filter(|&bin| {
            let left = smoothed[(bin + AZIMUTH_BIN_COUNT - 1) % AZIMUTH_BIN_COUNT];
            let right = smoothed[(bin + 1) % AZIMUTH_BIN_COUNT];
            smoothed[bin] > 0.0 && smoothed[bin] > left && smoothed[bin] >= right
        })
        .count();

    mode_count.max(1)
}

fn angular_distance(a: f32, b: f32) -> f32 {
    let difference = (a - b).rem_euclid(TAU);
    difference.min(TAU - difference)
}

fn find_nearest_center(azimuth: f32, centers: &[f32]) -> usize {
    centers
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            angular_distance(azimuth, **a).total_cmp(&angular_distance(azimuth, **b))
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

/// Weighted circular mean and the mean resultant length (1 = one direction, 0 = spread over the circle).
fn circular_mean_and_concentration(
    azimuths: impl Iterator<Item = (f32, f32)>,
) -> Option<(f32, f32)> {
    let (sin_sum, cos_sum, weight_sum) =
        azimuths.fold((0.0f32, 0.0f32, 0.0f32), |(s, c, n), (azimuth, weight)| {
            (
                s + azimuth.sin() * weight,
                c + azimuth.cos() * weight,
                n + weight,
            )
        });
    (weight_sum > 0.0).then(|| (sin_sum.atan2(cos_sum), sin_sum.hypot(cos_sum) / weight_sum))
}

fn compute_circular_mean(azimuths: impl Iterator<Item = (f32, f32)>) -> Option<f32> {
    circular_mean_and_concentration(azimuths).map(|(mean, _)| mean)
}

/// Start and angular extent of the occupied arc: the full circle when no bin is empty, otherwise the
/// complement of the widest empty gap, so every initial center lands on populated azimuths.
fn occupied_azimuth_arc(histogram: &[f32]) -> (f32, f32) {
    let bin_width = TAU / AZIMUTH_BIN_COUNT as f32;
    let empty = |bin: usize| histogram[bin % AZIMUTH_BIN_COUNT] <= 0.0;
    let Some(first_filled) = (0..AZIMUTH_BIN_COUNT).find(|&bin| !empty(bin)) else {
        return (0.0, TAU);
    };

    let mut widest_gap: Option<(usize, usize)> = None;
    let mut bin = first_filled;
    while bin < first_filled + AZIMUTH_BIN_COUNT {
        if !empty(bin) {
            bin += 1;
            continue;
        }
        let gap_start = bin;
        while bin < first_filled + AZIMUTH_BIN_COUNT && empty(bin) {
            bin += 1;
        }
        let gap_length = bin - gap_start;
        if widest_gap.is_none_or(|(_, length)| gap_length > length) {
            widest_gap = Some((gap_start, gap_length));
        }
    }

    match widest_gap {
        None => (0.0, TAU),
        Some((gap_start, gap_length)) => {
            let arc_start = (gap_start + gap_length) as f32 * bin_width;
            let arc_extent = (AZIMUTH_BIN_COUNT - gap_length) as f32 * bin_width;
            (arc_start, arc_extent)
        }
    }
}

fn cluster_azimuths(samples: &[AzimuthSample], count: usize) -> Vec<usize> {
    let histogram = smoothed_azimuth_histogram(samples);
    let (arc_start, arc_extent) = occupied_azimuth_arc(&histogram);
    let mut centers: Vec<f32> = (0..count)
        .map(|index| arc_start + arc_extent * (index as f32 + 0.5) / count as f32)
        .collect();
    let mut columns = vec![0usize; samples.len()];

    for _ in 0..KMEANS_ITERATION_COUNT {
        for (column, sample) in columns.iter_mut().zip(samples) {
            *column = find_nearest_center(sample.azimuth, &centers);
        }

        for (center_index, center) in centers.iter_mut().enumerate() {
            let members = samples
                .iter()
                .zip(&columns)
                .filter(|(_, &column)| column == center_index)
                .map(|(sample, _)| (sample.azimuth, sample.weight));
            if let Some(mean) = compute_circular_mean(members) {
                *center = mean;
            }
        }
    }

    columns
}

fn group_into_columns(assigned: &[(usize, Vec<usize>)], column_count: usize) -> Vec<Vec<usize>> {
    let mut columns = vec![Vec::new(); column_count];
    for (column, frame_indices) in assigned {
        columns[*column].extend(frame_indices.iter().copied());
    }
    columns.retain(|members| !members.is_empty());
    columns
}

fn split_skirt_columns(frames: &[VertexFrame], params: &ChainGenParams) -> Vec<Vec<usize>> {
    let samples: Vec<AzimuthSample> = frames
        .iter()
        .map(|frame| AzimuthSample {
            azimuth: frame.azimuth,
            weight: 1.0,
        })
        .collect();
    let column_count = match params.settings.column_count as usize {
        0 => count_azimuth_modes(&samples),
        count => count,
    };
    let assigned: Vec<(usize, Vec<usize>)> = frames
        .iter()
        .enumerate()
        .map(|(frame_index, frame)| {
            let column = azimuth_bin(frame.azimuth + PI / column_count as f32, column_count);
            (column, vec![frame_index])
        })
        .collect();
    group_into_columns(&assigned, column_count)
}

fn find_component_root(parent: &mut [usize], vertex: usize) -> usize {
    let mut root = vertex;
    while parent[root] != root {
        root = parent[root];
    }
    let mut current = vertex;
    while parent[current] != root {
        let next = parent[current];
        parent[current] = root;
        current = next;
    }
    root
}

/// Connected component label per vertex: the smallest vertex index of its component.
pub fn component_labels(input: &ChainGenInput) -> Vec<usize> {
    let mut parent: Vec<usize> = (0..input.positions.len()).collect();
    for triangle in input.triangles {
        for edge in [(triangle[0], triangle[1]), (triangle[1], triangle[2])] {
            let a = find_component_root(&mut parent, edge.0 as usize);
            let b = find_component_root(&mut parent, edge.1 as usize);
            parent[a.max(b)] = a.min(b);
        }
    }
    (0..input.positions.len())
        .map(|vertex| find_component_root(&mut parent, vertex))
        .collect()
}

fn label_connected_components(input: &ChainGenInput, frames: &[VertexFrame]) -> Vec<usize> {
    let labels = component_labels(input);
    frames
        .iter()
        .map(|frame| labels[frame.vertex as usize])
        .collect()
}

/// Strands below the root band, one column per azimuth cluster; a connected component never splits,
/// and a component spread around the head (the scalp shell) stays on the root.
fn split_hair_columns(
    input: &ChainGenInput,
    frames: &[VertexFrame],
    band: RootBand,
    params: &ChainGenParams,
) -> Vec<Vec<usize>> {
    let strand_frames: Vec<usize> = (0..frames.len())
        .filter(|&index| frames[index].progress > band.progress)
        .collect();
    let strand_samples: Vec<AzimuthSample> = strand_frames
        .iter()
        .map(|&index| AzimuthSample {
            azimuth: frames[index].azimuth,
            weight: 1.0,
        })
        .collect();
    let column_count = match params.settings.column_count as usize {
        0 => count_azimuth_modes(&strand_samples),
        count => count,
    };

    let component_labels = label_connected_components(input, frames);
    let component_count = component_labels.iter().collect::<BTreeSet<_>>().len();
    if component_count < 2 {
        let columns = cluster_azimuths(&strand_samples, column_count);
        let assigned: Vec<(usize, Vec<usize>)> = columns
            .into_iter()
            .zip(strand_frames)
            .map(|(column, frame_index)| (column, vec![frame_index]))
            .collect();
        return group_into_columns(&assigned, column_count);
    }

    let mut strands_by_component: Vec<(usize, Vec<usize>)> = Vec::new();
    for &frame_index in &strand_frames {
        let label = component_labels[frame_index];
        match strands_by_component.binary_search_by_key(&label, |(l, _)| *l) {
            Ok(slot) => strands_by_component[slot].1.push(frame_index),
            Err(slot) => strands_by_component.insert(slot, (label, vec![frame_index])),
        }
    }
    let strands_by_component: Vec<(f32, Vec<usize>)> = strands_by_component
        .into_iter()
        .filter_map(|(_, members)| {
            let (mean, concentration) =
                circular_mean_and_concentration(members.iter().map(|&i| (frames[i].azimuth, 1.0)))?;
            (concentration >= MIN_STRAND_AZIMUTH_CONCENTRATION).then_some((mean, members))
        })
        .collect();
    let component_samples: Vec<AzimuthSample> = strands_by_component
        .iter()
        .map(|(azimuth, members)| AzimuthSample {
            azimuth: *azimuth,
            weight: members.len() as f32,
        })
        .collect();
    let columns = cluster_azimuths(&component_samples, column_count);
    let assigned: Vec<(usize, Vec<usize>)> = columns
        .into_iter()
        .zip(strands_by_component)
        .map(|(column, (_, members))| (column, members))
        .collect();
    group_into_columns(&assigned, column_count)
}

fn build_centerline(
    frames: &[&VertexFrame],
    positions: &[Vector3<f32>],
    joint_count: usize,
) -> (Vec<Vector3<f32>>, Vector3<f32>) {
    let progresses = frames.iter().map(|frame| frame.progress);
    let min_progress = progresses.clone().reduce(f32::min).unwrap_or_default();
    let max_progress = progresses.reduce(f32::max).unwrap_or_default();
    let band_width = (max_progress - min_progress) / joint_count as f32;

    let mut band_sums = vec![(Vector3::new(0.0, 0.0, 0.0), 0usize); joint_count];
    for frame in frames {
        let band = if band_width > 0.0 {
            (((frame.progress - min_progress) / band_width) as usize).min(joint_count - 1)
        } else {
            0
        };
        band_sums[band].0 += positions[frame.vertex as usize];
        band_sums[band].1 += 1;
    }

    let centroids = band_sums
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(sum, count)| sum / count as f32)
        .collect();
    let tail_frame = frames
        .iter()
        .max_by(|a, b| a.progress.total_cmp(&b.progress))
        .expect("build_centerline requires at least one frame");

    (centroids, positions[tail_frame.vertex as usize])
}

pub(crate) fn resample_polyline(points: &[Vector3<f32>], count: usize) -> Vec<Vector3<f32>> {
    let segment_lengths: Vec<f32> = points
        .windows(2)
        .map(|segment| (segment[1] - segment[0]).magnitude())
        .collect();
    let total_length: f32 = segment_lengths.iter().sum();

    (0..count)
        .map(|index| {
            let mut remaining = total_length * index as f32 / count as f32;
            for (segment, &length) in points.windows(2).zip(&segment_lengths) {
                if remaining <= length && length > 0.0 {
                    return segment[0] + (segment[1] - segment[0]) * (remaining / length);
                }
                remaining -= length;
            }
            points
                .last()
                .copied()
                .unwrap_or(Vector3::new(0.0, 0.0, 0.0))
        })
        .collect()
}

fn build_chain(
    column: usize,
    frames: &[&VertexFrame],
    input: &ChainGenInput,
    params: &ChainGenParams,
) -> GeneratedChain {
    let joint_count = params.settings.joint_count as usize;
    let (mut polyline, tail) = build_centerline(frames, input.positions, joint_count);
    polyline.push(tail);

    let joints = resample_polyline(&polyline, joint_count)
        .into_iter()
        .enumerate()
        .map(|(joint, head)| GeneratedJoint {
            name: joint_name(params.role, column, joint),
            head,
        })
        .collect();
    let mut vertices: Vec<u32> = frames.iter().map(|frame| frame.vertex).collect();
    vertices.sort_unstable();

    GeneratedChain {
        column,
        joints,
        tail,
        vertices,
    }
}

fn attachment_or_target<'a>(
    input: &ChainGenInput<'a>,
    vertices: &[u32],
) -> Cow<'a, [Vector3<f32>]> {
    if input.attachment_positions.is_empty() {
        Cow::Owned(
            vertices
                .iter()
                .map(|&vertex| input.positions[vertex as usize])
                .collect(),
        )
    } else {
        Cow::Borrowed(input.attachment_positions)
    }
}

pub fn place_chains(
    input: &ChainGenInput,
    params: &ChainGenParams,
) -> anyhow::Result<Vec<GeneratedChain>> {
    if params.settings.joint_count == 0 {
        anyhow::bail!("chain generation needs at least one joint");
    }
    let vertices = collect_target_vertices(input);
    if vertices.is_empty() {
        anyhow::bail!("chain generation found no target vertices");
    }

    let root_band = hair_root_band(input, params);
    let azimuth_center = root_band.map_or(input.root_position, |band| band.center);
    let frames = compute_vertex_frames(input, &vertices, azimuth_center);
    let columns = match root_band {
        Some(band) => split_hair_columns(input, &frames, band, params),
        None => split_skirt_columns(&frames, params),
    };
    if columns.is_empty() {
        anyhow::bail!("chain generation found no vertex below the root band");
    }
    let chains = columns
        .into_iter()
        .enumerate()
        .map(|(column, members)| {
            let column_frames: Vec<&VertexFrame> = members
                .iter()
                .map(|&frame_index| &frames[frame_index])
                .collect();
            build_chain(column, &column_frames, input, params)
        })
        .collect();

    Ok(chains)
}

struct ChainArc {
    arc: Vec<f32>,
    total_length: f32,
}

impl ChainArc {
    fn new(chain: &GeneratedChain) -> Self {
        let mut arc = vec![0.0];
        let mut current = chain.joints[0].head;
        for joint in &chain.joints[1..] {
            arc.push(arc.last().copied().unwrap() + (joint.head - current).magnitude());
            current = joint.head;
        }
        let total_length = arc.last().copied().unwrap() + (chain.tail - current).magnitude();
        Self { arc, total_length }
    }

    fn first_joint_length(&self) -> f32 {
        match self.arc.get(1) {
            Some(second) => second - self.arc[0],
            None => self.total_length,
        }
    }
}

struct PolylineHit {
    distance_squared: f32,
    arc_position: f32,
}

fn project_to_polyline(point: Vector3<f32>, chain: &GeneratedChain, arc: &ChainArc) -> PolylineHit {
    let nearest = chain
        .joints
        .iter()
        .enumerate()
        .map(|(i, joint)| {
            let (next_pos, next_arc) = match chain.joints.get(i + 1) {
                Some(next_joint) => (next_joint.head, arc.arc[i + 1]),
                None => (chain.tail, arc.total_length),
            };
            let (t, projected) = line_projection(point, joint.head, next_pos);
            PolylineHit {
                distance_squared: (point - projected).magnitude2(),
                arc_position: arc.arc[i] + t * (next_arc - arc.arc[i]),
            }
        })
        .min_by(|a, b| a.distance_squared.total_cmp(&b.distance_squared));

    match nearest {
        Some(hit) => PolylineHit {
            arc_position: hit.arc_position.clamp(0.0, arc.total_length),
            ..hit
        },
        None => PolylineHit {
            distance_squared: 0.0,
            arc_position: 0.0,
        },
    }
}

fn line_projection(point: Vector3<f32>, a: Vector3<f32>, b: Vector3<f32>) -> (f32, Vector3<f32>) {
    let ab = b - a;
    let ap = point - a;
    let len_sq = ab.magnitude2();
    if len_sq < 1e-10 {
        return (0.0, a);
    }
    let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);
    (t, a + ab * t)
}

fn joint_weights_at_arc(s: f32, arc: &ChainArc) -> Vec<(usize, f32)> {
    let last_joint_index = arc.arc.len() - 1;

    if s >= arc.total_length {
        return vec![(last_joint_index, 1.0)];
    }

    for (i, &arc_i) in arc.arc.iter().enumerate() {
        if i == last_joint_index {
            return vec![(i, 1.0)];
        }
        let arc_next = arc.arc[i + 1];
        if s < arc_next {
            let segment_length = arc_next - arc_i;
            if segment_length < 1e-10 {
                return vec![(i, 1.0)];
            }
            let f = (s - arc_i) / segment_length;
            return vec![(i, 1.0 - f), (i + 1, f)];
        }
    }

    vec![(last_joint_index, 1.0)]
}

fn chain_influences(
    chain_index: usize,
    arc_position: f32,
    arc: &ChainArc,
    scale: f32,
) -> impl Iterator<Item = (InfluenceTarget, f32)> {
    joint_weights_at_arc(arc_position, arc)
        .into_iter()
        .map(move |(joint, weight)| {
            (
                InfluenceTarget::Joint {
                    chain: chain_index,
                    joint,
                },
                weight * scale,
            )
        })
}

fn find_nearest_other_chain(
    point: Vector3<f32>,
    own_chain: usize,
    chains: &[GeneratedChain],
    arcs: &[ChainArc],
) -> Option<(usize, PolylineHit)> {
    chains
        .iter()
        .zip(arcs)
        .enumerate()
        .filter(|(index, _)| *index != own_chain)
        .map(|(index, (chain, arc))| (index, project_to_polyline(point, chain, arc)))
        .min_by(|a, b| a.1.distance_squared.total_cmp(&b.1.distance_squared))
}

/// Share of the neighbouring column: 0 beyond `width` from the boundary, 0.5 at equal distance.
fn column_blend_share(own_distance: f32, other_distance: f32, width: f32) -> f32 {
    ((own_distance - other_distance + width) / (2.0 * width)).clamp(0.0, 1.0)
}

fn apply_root_blend(influences: &mut Vec<(InfluenceTarget, f32)>, arc_position: f32, width: f32) {
    if arc_position >= width {
        return;
    }
    let chain_share = arc_position / width;
    for influence in influences.iter_mut() {
        influence.1 *= chain_share;
    }
    influences.push((InfluenceTarget::Root, 1.0 - chain_share));
}

fn normalize_influences(influences: &mut Vec<(InfluenceTarget, f32)>) {
    influences.retain(|(_, weight)| *weight > 0.0);
    influences.sort_by(|a, b| b.1.total_cmp(&a.1));
    influences.truncate(MAX_INFLUENCES_PER_VERTEX);

    let sum: f32 = influences.iter().map(|(_, w)| w).sum();
    if sum > 1e-8 {
        for influence in influences.iter_mut() {
            influence.1 /= sum;
        }
    }
}

fn vertex_influences(
    vertex: u32,
    chain_index: usize,
    input: &ChainGenInput,
    params: &ChainGenParams,
    chains: &[GeneratedChain],
    arcs: &[ChainArc],
) -> VertexInfluences {
    let point = input.positions[vertex as usize];
    let own = project_to_polyline(point, &chains[chain_index], &arcs[chain_index]);

    let blend_width = params.settings.column_blend_width;
    let neighbour = (blend_width > 0.0)
        .then(|| find_nearest_other_chain(point, chain_index, chains, arcs))
        .flatten()
        .map(|(other_index, hit)| {
            let share = column_blend_share(
                own.distance_squared.sqrt(),
                hit.distance_squared.sqrt(),
                blend_width,
            );
            (other_index, hit, share)
        })
        .filter(|(_, _, share)| *share > 0.0);
    let own_share = neighbour.as_ref().map_or(1.0, |(_, _, share)| 1.0 - share);

    let mut influences: Vec<(InfluenceTarget, f32)> =
        chain_influences(chain_index, own.arc_position, &arcs[chain_index], own_share).collect();
    if let Some((other_index, hit, share)) = neighbour {
        influences.extend(chain_influences(
            other_index,
            hit.arc_position,
            &arcs[other_index],
            share,
        ));
    }

    let root_blend_width = match params.settings.root_blend_width {
        width if width > 0.0 => width,
        _ => arcs[chain_index].first_joint_length(),
    };
    apply_root_blend(&mut influences, own.arc_position, root_blend_width);
    normalize_influences(&mut influences);

    VertexInfluences { vertex, influences }
}

pub fn setting_names() -> Vec<&'static str> {
    CHAIN_GEN_SCALAR_PARAMS.iter().map(|p| p.name).collect()
}

pub fn apply_setting_assignments(
    settings: &mut ChainGenSettings,
    assignments: &[(String, f32)],
) -> anyhow::Result<()> {
    for (name, value) in assignments {
        let param = find_scalar_param(&CHAIN_GEN_SCALAR_PARAMS, name).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown chain generation setting \"{name}\"; valid: {}",
                setting_names().join(", ")
            )
        })?;
        (param.set)(settings, *value);
    }
    Ok(())
}

pub fn compute_chain_weights(
    input: &ChainGenInput,
    params: &ChainGenParams,
    chains: &[GeneratedChain],
) -> Vec<VertexInfluences> {
    let arcs: Vec<ChainArc> = chains.iter().map(ChainArc::new).collect();

    let mut result: Vec<VertexInfluences> = chains
        .iter()
        .enumerate()
        .flat_map(|(chain_index, chain)| {
            chain
                .vertices
                .iter()
                .map(move |&vertex| (chain_index, vertex))
        })
        .map(|(chain_index, vertex)| {
            vertex_influences(vertex, chain_index, input, params, chains, &arcs)
        })
        .collect();

    let chained: BTreeSet<u32> = chains
        .iter()
        .flat_map(|chain| chain.vertices.iter().copied())
        .collect();
    result.extend(
        collect_target_vertices(input)
            .into_iter()
            .filter(|vertex| !chained.contains(vertex))
            .map(|vertex| VertexInfluences {
                vertex,
                influences: vec![(InfluenceTarget::Root, 1.0)],
            }),
    );

    result.sort_by_key(|a| a.vertex);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_straight_strand() -> (Vec<Vector3<f32>>, Vec<[u32; 3]>) {
        let positions = (1..=5)
            .map(|step| Vector3::new(0.0, -0.1 * step as f32, 0.0))
            .collect();
        (positions, vec![[0, 1, 2], [2, 3, 4]])
    }

    fn build_params(role: ChainRole, joint_count: u32, column_count: u32) -> ChainGenParams {
        ChainGenParams {
            role,
            settings: ChainGenSettings {
                joint_count,
                column_count,
                ..ChainGenSettings::default()
            },
        }
    }

    fn build_hair_params(joint_count: u32) -> ChainGenParams {
        build_params(ChainRole::Hair, joint_count, 1)
    }

    fn head_attachment() -> Vec<Vector3<f32>> {
        (0..8)
            .map(|i| {
                let angle = TAU * i as f32 / 8.0;
                Vector3::new(0.2 * angle.cos(), 0.1, 0.2 * angle.sin())
            })
            .collect()
    }

    fn build_cap_with_two_cards() -> (Vec<Vector3<f32>>, Vec<[u32; 3]>) {
        let mut positions = Vec::new();
        let mut triangles = Vec::new();

        let cap_ring = |y: f32, radius: f32| -> Vec<Vector3<f32>> {
            (0..12)
                .map(|i| {
                    let angle = TAU * i as f32 / 12.0;
                    Vector3::new(radius * angle.cos(), y, radius * angle.sin())
                })
                .collect()
        };
        for (upper, lower) in [(0.25f32, 0.1), (0.15, 0.2)]
            .iter()
            .zip([(0.15f32, 0.2), (0.05, 0.22)])
        {
            let base = positions.len() as u32;
            positions.extend(cap_ring(upper.0, upper.1));
            positions.extend(cap_ring(lower.0, lower.1));
            for i in 0..12u32 {
                let next = (i + 1) % 12;
                triangles.push([base + i, base + next, base + 12 + i]);
                triangles.push([base + next, base + 12 + next, base + 12 + i]);
            }
        }

        for angle in [PI * 0.75, -PI * 0.75] {
            let base = positions.len() as u32;
            let (sin, cos) = angle.sin_cos();
            for step in 0..5 {
                let y = 0.05 - 0.08 * step as f32;
                positions.push(Vector3::new(0.21 * cos, y, 0.21 * sin));
                positions.push(Vector3::new(
                    0.21 * cos + 0.01 * sin,
                    y,
                    0.21 * sin - 0.01 * cos,
                ));
            }
            for step in 0..4u32 {
                let a = base + 2 * step;
                triangles.push([a, a + 1, a + 2]);
                triangles.push([a + 1, a + 3, a + 2]);
            }
        }
        (positions, triangles)
    }

    #[test]
    fn straight_strand_places_heads_at_equal_spacing() {
        let head = head_attachment();
        let (positions, triangles) = build_straight_strand();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
        };

        let chains = place_chains(&input, &build_hair_params(4)).unwrap();

        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].vertices, vec![0, 1, 2, 3, 4]);
        assert_eq!(chains[0].joints.len(), 4);
        assert_eq!(chains[0].joints[0].name, joint_name(ChainRole::Hair, 0, 0));
        for (joint_index, joint) in chains[0].joints.iter().enumerate() {
            let expected = Vector3::new(0.0, -0.1 - 0.1 * joint_index as f32, 0.0);
            assert!(
                (joint.head - expected).magnitude() < 1e-4,
                "joint {joint_index}: {:?}",
                joint.head
            );
        }
        assert!((chains[0].tail - Vector3::new(0.0, -0.5, 0.0)).magnitude() < 1e-4);
    }

    #[test]
    fn zero_joint_count_is_rejected() {
        let head = head_attachment();
        let (positions, triangles) = build_straight_strand();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
        };

        assert!(place_chains(&input, &build_hair_params(0)).is_err());
    }

    #[test]
    fn skirt_column_split_by_azimuth() {
        let mut positions: Vec<Vector3<f32>> = Vec::new();
        let mut triangles: Vec<[u32; 3]> = Vec::new();
        let ring_count = 2;
        let samples_per_ring = 24;
        let radius = 0.3;
        let y_offsets = [-0.2f32, -0.6];

        for &y in &y_offsets {
            for i in 0..samples_per_ring {
                let angle = TAU * i as f32 / samples_per_ring as f32;
                positions.push(Vector3::new(radius * angle.cos(), y, radius * angle.sin()));
            }
        }

        for ring in 0..ring_count {
            let base = (ring * samples_per_ring) as u32;
            for i in 0..samples_per_ring {
                let a = base + i as u32;
                let b = base + (i + 1) as u32 % samples_per_ring as u32;
                if ring < ring_count - 1 {
                    let c = base + samples_per_ring as u32 + i as u32;
                    let d =
                        base + samples_per_ring as u32 + (i + 1) as u32 % samples_per_ring as u32;
                    triangles.push([a, b, c]);
                    triangles.push([b, d, c]);
                }
            }
        }

        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &[],
        };
        let params = build_params(ChainRole::Skirt, 2, 8);

        let chains = place_chains(&input, &params).unwrap();
        assert_eq!(chains.len(), 8);

        let mut angles: Vec<f32> = chains
            .iter()
            .map(|chain| {
                let head = chain.joints[0].head;
                head.x.atan2(head.z)
            })
            .collect();
        angles.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let wrapped_first = angles[0] + TAU;
        let expected_gap = TAU / 8.0;
        for (index, next) in angles.iter().skip(1).chain([&wrapped_first]).enumerate() {
            let gap = next - angles[index];
            assert!(
                (gap - expected_gap).abs() < 1f32.to_radians(),
                "azimuth gap {gap:.4} vs expected {expected_gap:.4}"
            );
        }
    }

    #[test]
    fn hair_azimuth_mode_detection() {
        let head = head_attachment();
        let mut positions: Vec<Vector3<f32>> = Vec::new();
        let mut triangles: Vec<[u32; 3]> = Vec::new();

        let mode_centers = [0.5f32, 3.5];
        let count_per_mode = 6;

        for &center in &mode_centers {
            for i in 0..count_per_mode {
                let angle = center + (i as f32 - 2.5) * 0.05;
                let y = -0.1 * (i as f32 + 1.0);
                positions.push(Vector3::new(angle.cos() * 0.3, y, angle.sin() * 0.3));
            }
        }

        for mode in 0..mode_centers.len() {
            let base = (mode * count_per_mode) as u32;
            for i in 0..count_per_mode - 2 {
                triangles.push([base + i as u32, base + i as u32 + 1, base + i as u32 + 2]);
            }
        }

        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
        };
        let params = build_params(ChainRole::Hair, 2, 0);

        let chains = place_chains(&input, &params).unwrap();
        assert_eq!(chains.len(), 2);
    }

    #[test]
    fn weights_sum_to_one() {
        let head = head_attachment();
        let (positions, triangles) = build_straight_strand();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
        };

        let chains = place_chains(&input, &build_hair_params(4)).unwrap();
        let weights = compute_chain_weights(&input, &build_hair_params(4), &chains);

        for entry in &weights {
            let sum: f32 = entry.influences.iter().map(|(_, w)| w).sum();
            assert!(
                (sum - 1.0).abs() < 1e-5,
                "vertex {}: weight sum = {:.6}",
                entry.vertex,
                sum
            );
        }
    }

    #[test]
    fn midpoint_vertex_has_equal_joint_weights() {
        let head = head_attachment();
        let (mut positions, triangles) = build_straight_strand();
        let mut chains = place_chains(
            &ChainGenInput {
                positions: &positions,
                triangles: &triangles,
                root_position: Vector3::new(0.0, 0.0, 0.0),
                attachment_positions: &head,
            },
            &build_hair_params(4),
        )
        .unwrap();

        let midpoint_vertex = positions.len() as u32;
        positions.push(Vector3::new(0.0, -0.25, 0.0));
        chains[0].vertices.push(midpoint_vertex);
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
        };
        let weights = compute_chain_weights(&input, &build_hair_params(4), &chains);

        let midpoint_entry = weights
            .iter()
            .find(|e| e.vertex == midpoint_vertex)
            .expect("midpoint vertex not found");
        let mut joint1_weight = 0.0f32;
        let mut joint2_weight = 0.0f32;
        for (target, w) in &midpoint_entry.influences {
            match *target {
                InfluenceTarget::Joint { chain: _, joint } => match joint {
                    1 => joint1_weight = *w,
                    2 => joint2_weight = *w,
                    _ => {}
                },
                InfluenceTarget::Root => {}
            }
        }
        assert!(
            (joint1_weight - 0.5).abs() < 1e-4,
            "joint1 weight = {:.6}, expected ~0.5",
            joint1_weight
        );
        assert!(
            (joint2_weight - 0.5).abs() < 1e-4,
            "joint2 weight = {:.6}, expected ~0.5",
            joint2_weight
        );
    }

    #[test]
    fn first_vertex_has_full_root_weight() {
        let head = head_attachment();
        let (positions, triangles) = build_straight_strand();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
        };

        let chains = place_chains(&input, &build_hair_params(4)).unwrap();
        let weights = compute_chain_weights(&input, &build_hair_params(4), &chains);

        let first_entry = weights
            .iter()
            .find(|e| e.vertex == 0)
            .expect("vertex 0 not found");
        let mut root_weight = 0.0f32;
        for (target, w) in &first_entry.influences {
            if let InfluenceTarget::Root = target {
                root_weight = *w;
            }
        }
        assert!(
            (root_weight - 1.0).abs() < 1e-5,
            "vertex 0 root weight = {:.6}, expected 1.0",
            root_weight
        );
    }

    #[test]
    fn hair_joints_y_monotonically_decreasing() {
        let head = head_attachment();
        let positions: Vec<Vector3<f32>> = (0..6)
            .map(|i| Vector3::new(0.0, 0.2 - 0.1 * i as f32, 0.0))
            .collect();
        let triangles = vec![[0u32, 1, 2], [2, 3, 4], [4, 5, 0]];
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
        };

        let chains = place_chains(&input, &build_hair_params(3)).unwrap();
        assert_eq!(chains.len(), 1);
        let joints = &chains[0].joints;

        for i in 1..joints.len() {
            assert!(
                joints[i].head.y < joints[i - 1].head.y,
                "joint[{}].head.y ({:.4}) >= joint[{}].head.y ({:.4})",
                i,
                joints[i].head.y,
                i - 1,
                joints[i - 1].head.y
            );
        }
    }

    #[test]
    fn reversed_vertex_order_same_joints() {
        let head = head_attachment();
        let positions: Vec<Vector3<f32>> = (0..6)
            .map(|i| Vector3::new(0.0, 0.2 - 0.1 * i as f32, 0.0))
            .collect();
        let triangles = vec![[0u32, 1, 2], [2, 3, 4], [4, 5, 0]];
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
        };

        let chains = place_chains(&input, &build_hair_params(2)).unwrap();
        let original_joints: Vec<Vector3<f32>> = chains[0].joints.iter().map(|j| j.head).collect();

        let reversed_positions: Vec<Vector3<f32>> = positions.iter().rev().cloned().collect();
        let reversed_triangles = vec![[5u32, 4, 3], [3, 2, 0], [0, 1, 5]];
        let reversed_input = ChainGenInput {
            positions: &reversed_positions,
            triangles: &reversed_triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
        };

        let reversed_chains = place_chains(&reversed_input, &build_hair_params(2)).unwrap();
        let reversed_joints: Vec<Vector3<f32>> =
            reversed_chains[0].joints.iter().map(|j| j.head).collect();

        assert_eq!(original_joints.len(), reversed_joints.len());
        for (i, (a, b)) in original_joints.iter().zip(&reversed_joints).enumerate() {
            let diff = (a.x - b.x).hypot(a.y - b.y).hypot(a.z - b.z);
            assert!(
                diff < 1e-5,
                "joint[{}] head differs: original={:?}, reversed={:?}, diff={:.6}",
                i,
                a,
                b,
                diff
            );
        }
    }

    fn build_two_strands() -> (Vec<Vector3<f32>>, Vec<[u32; 3]>) {
        let mut positions = Vec::new();
        let mut triangles = Vec::new();
        for (strand, x) in [(0u32, -0.05f32), (1, 0.05)] {
            let base = positions.len() as u32;
            for step in 0..4 {
                positions.push(Vector3::new(x, -0.1 * step as f32 - 0.1, 0.0));
                positions.push(Vector3::new(x, -0.1 * step as f32 - 0.1, 0.02));
            }
            for step in 0..3u32 {
                let a = base + 2 * step;
                triangles.push([a, a + 1, a + 2]);
                triangles.push([a + 1, a + 3, a + 2]);
            }
            let _ = strand;
        }
        (positions, triangles)
    }

    fn total_weight_on_chain(entry: &VertexInfluences, chain: usize) -> f32 {
        entry
            .influences
            .iter()
            .filter(|(target, _)| matches!(target, InfluenceTarget::Joint { chain: c, .. } if *c == chain))
            .map(|(_, weight)| weight)
            .sum()
    }

    #[test]
    fn zero_column_blend_keeps_every_vertex_on_its_own_column() {
        let (positions, triangles) = build_two_strands();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &[],
        };
        let params = build_params(ChainRole::Skirt, 3, 2);

        let chains = place_chains(&input, &params).unwrap();
        let weights = compute_chain_weights(&input, &params, &chains);

        assert_eq!(chains.len(), 2);
        for entry in &weights {
            let chain_count = (0..2)
                .filter(|&chain| total_weight_on_chain(entry, chain) > 0.0)
                .count();
            assert!(chain_count <= 1, "vertex {} spans columns", entry.vertex);
        }
    }

    #[test]
    fn column_blend_shares_boundary_vertices_between_neighbouring_columns() {
        let (mut positions, triangles) = build_two_strands();
        let input_for_chains = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &[],
        };
        let mut params = build_params(ChainRole::Skirt, 3, 2);
        let mut chains = place_chains(&input_for_chains, &params).unwrap();

        let boundary_vertex = positions.len() as u32;
        positions.push(Vector3::new(0.0, -0.25, 0.01));
        chains[0].vertices.push(boundary_vertex);
        params.settings.column_blend_width = 0.1;
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &[],
        };

        let weights = compute_chain_weights(&input, &params, &chains);

        let entry = weights
            .iter()
            .find(|e| e.vertex == boundary_vertex)
            .unwrap();
        let own = total_weight_on_chain(entry, 0);
        let other = total_weight_on_chain(entry, 1);
        assert!((own - other).abs() < 1e-4, "own {own} other {other}");
        assert!(entry.influences.len() <= MAX_INFLUENCES_PER_VERTEX);
        let sum: f32 = entry.influences.iter().map(|(_, w)| w).sum();
        assert!((sum - 1.0).abs() < 1e-5);

        let far_vertex = weights.iter().find(|e| e.vertex == 0).unwrap();
        assert_eq!(total_weight_on_chain(far_vertex, 1), 0.0);
    }

    #[test]
    fn cap_stays_on_root_and_each_card_is_one_column() {
        let (positions, triangles) = build_cap_with_two_cards();
        let head = head_attachment();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
        };
        let params = build_params(ChainRole::Hair, 3, 0);

        let chains = place_chains(&input, &params).unwrap();
        let weights = compute_chain_weights(&input, &params, &chains);

        assert_eq!(chains.len(), 2, "one column per card");
        let card_start = 48u32;
        for chain in &chains {
            assert!(
                chain.vertices.iter().all(|&v| v >= card_start),
                "cap vertex in a chain"
            );
            let first_card = (chain.vertices[0] - card_start) / 10;
            assert!(
                chain
                    .vertices
                    .iter()
                    .all(|&v| (v - card_start) / 10 == first_card),
                "card split across columns: {:?}",
                chain.vertices
            );
            for joint in &chain.joints {
                assert!(
                    joint.head.y < 0.1,
                    "joint above the root band: {:?}",
                    joint.head
                );
            }
        }
        for entry in weights.iter().filter(|e| e.vertex < card_start) {
            assert_eq!(entry.influences, vec![(InfluenceTarget::Root, 1.0)]);
        }
    }

    #[test]
    fn root_band_offset_moves_the_band_down() {
        let (positions, triangles) = build_cap_with_two_cards();
        let head = head_attachment();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
        };
        let mut params = build_params(ChainRole::Hair, 3, 2);
        let baseline = place_chains(&input, &params).unwrap();
        params.settings.root_band_offset = 0.2;

        let shifted = place_chains(&input, &params).unwrap();

        let strand_count =
            |chains: &[GeneratedChain]| -> usize { chains.iter().map(|c| c.vertices.len()).sum() };
        assert!(strand_count(&shifted) < strand_count(&baseline));
        assert!(shifted[0].joints[0].head.y < baseline[0].joints[0].head.y);
    }

    #[test]
    fn root_band_without_attachment_uses_the_widest_slice_of_the_target() {
        let (positions, triangles) = build_cap_with_two_cards();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &[],
        };

        let chains = place_chains(&input, &build_params(ChainRole::Hair, 3, 2)).unwrap();

        assert_eq!(chains.len(), 2);
        assert!(chains.iter().all(|c| c.vertices.iter().all(|&v| v >= 48)));
    }

    #[test]
    fn component_labels_separate_the_cap_from_each_card() {
        let (positions, triangles) = build_cap_with_two_cards();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &[],
        };

        let labels = component_labels(&input);

        assert_eq!(labels.len(), positions.len());
        let distinct: BTreeSet<usize> = labels.iter().copied().collect();
        assert_eq!(distinct.len(), 4, "two cap rings and two cards");
        assert!(labels[48..58].iter().all(|&l| l == 48));
        assert!(labels[58..68].iter().all(|&l| l == 58));
    }

    #[test]
    fn hair_root_band_reports_the_equator_and_the_offset_band() {
        let (positions, triangles) = build_cap_with_two_cards();
        let head = head_attachment();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
        };
        let mut params = build_params(ChainRole::Hair, 3, 2);
        params.settings.root_band_offset = 0.05;

        let band = hair_root_band(&input, &params).unwrap();

        assert!((band.progress - band.equator_progress - 0.05).abs() < 1e-6);
        assert!(
            (band.center.y - 0.1).abs() < 1e-6,
            "ring height {:?}",
            band.center
        );
        assert!(hair_root_band(&input, &build_params(ChainRole::Skirt, 3, 2)).is_none());
    }
}
