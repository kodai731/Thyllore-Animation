use std::collections::BTreeSet;
use std::f32::consts::{PI, TAU};

use cgmath::{InnerSpace, Vector3};

use crate::garment::components::chain_gen::{
    joint_name, ChainGenInput, ChainGenParams, ChainRole, GeneratedChain, GeneratedJoint,
    InfluenceTarget, VertexInfluences,
};

const AZIMUTH_BIN_COUNT: usize = 36;
const KMEANS_ITERATION_COUNT: usize = 16;

#[derive(Debug, Clone, Copy)]
struct VertexFrame {
    vertex: u32,
    azimuth: f32,
    progress: f32,
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
    let up = up.normalize();
    let seed = if up.x.abs() < 0.9 {
        Vector3::unit_x()
    } else {
        Vector3::unit_z()
    };
    let reference = (seed - up * seed.dot(up)).normalize();
    (reference, up.cross(reference))
}

fn compute_vertex_frames(input: &ChainGenInput, vertices: &[u32]) -> Vec<VertexFrame> {
    let (reference, side) = build_azimuth_basis(input.up);
    let up_normalized = input.up.normalize();

    vertices
        .iter()
        .map(|&vertex| {
            let position = input.positions[vertex as usize];
            let offset = position - input.root_position;
            let progress = (input.root_position - position).dot(up_normalized);
            VertexFrame {
                vertex,
                azimuth: offset.dot(side).atan2(offset.dot(reference)),
                progress,
            }
        })
        .collect()
}

fn azimuth_bin(azimuth: f32, bin_count: usize) -> usize {
    let normalized = azimuth.rem_euclid(TAU) / TAU;
    ((normalized * bin_count as f32) as usize).min(bin_count - 1)
}

fn count_azimuth_modes(frames: &[VertexFrame]) -> usize {
    let mut histogram = [0usize; AZIMUTH_BIN_COUNT];
    for frame in frames {
        histogram[azimuth_bin(frame.azimuth, AZIMUTH_BIN_COUNT)] += 1;
    }

    let smoothed: Vec<f32> = (0..AZIMUTH_BIN_COUNT)
        .map(|bin| {
            let left = histogram[(bin + AZIMUTH_BIN_COUNT - 1) % AZIMUTH_BIN_COUNT];
            let right = histogram[(bin + 1) % AZIMUTH_BIN_COUNT];
            (left + histogram[bin] + right) as f32 / 3.0
        })
        .collect();

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

fn compute_circular_mean(azimuths: impl Iterator<Item = f32>) -> Option<f32> {
    let (sin_sum, cos_sum, count) = azimuths
        .fold((0.0f32, 0.0f32, 0usize), |(s, c, n), azimuth| {
            (s + azimuth.sin(), c + azimuth.cos(), n + 1)
        });
    (count > 0).then(|| sin_sum.atan2(cos_sum))
}

fn cluster_azimuths(frames: &[VertexFrame], count: usize) -> Vec<usize> {
    let start = frames.first().map_or(0.0, |frame| frame.azimuth);
    let mut centers: Vec<f32> = (0..count)
        .map(|index| start + TAU * index as f32 / count as f32)
        .collect();
    let mut columns = vec![0usize; frames.len()];

    for _ in 0..KMEANS_ITERATION_COUNT {
        for (column, frame) in columns.iter_mut().zip(frames) {
            *column = find_nearest_center(frame.azimuth, &centers);
        }

        for (center_index, center) in centers.iter_mut().enumerate() {
            let members = frames
                .iter()
                .zip(&columns)
                .filter(|(_, &column)| column == center_index)
                .map(|(frame, _)| frame.azimuth);
            if let Some(mean) = compute_circular_mean(members) {
                *center = mean;
            }
        }
    }

    columns
}

fn assign_skirt_columns(frames: &[VertexFrame], count: usize) -> Vec<usize> {
    frames
        .iter()
        .map(|frame| azimuth_bin(frame.azimuth + PI / count as f32, count))
        .collect()
}

fn split_columns(frames: &[VertexFrame], params: &ChainGenParams) -> Vec<Vec<usize>> {
    let column_count = match (params.role, params.column_count) {
        (ChainRole::Hair, 0) => count_azimuth_modes(frames),
        (ChainRole::Skirt, 0) => count_azimuth_modes(frames),
        (_, count) => count,
    };
    let assigned_columns = match params.role {
        ChainRole::Hair => cluster_azimuths(frames, column_count),
        ChainRole::Skirt => assign_skirt_columns(frames, column_count),
    };

    let mut columns = vec![Vec::new(); column_count];
    for (frame_index, &column) in assigned_columns.iter().enumerate() {
        columns[column].push(frame_index);
    }
    columns.retain(|members| !members.is_empty());
    columns
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

fn resample_polyline(points: &[Vector3<f32>], count: usize) -> Vec<Vector3<f32>> {
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
    let (mut polyline, tail) = build_centerline(frames, input.positions, params.joint_count);
    polyline.push(tail);

    let joints = resample_polyline(&polyline, params.joint_count)
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

pub fn place_chains(
    input: &ChainGenInput,
    params: &ChainGenParams,
) -> anyhow::Result<Vec<GeneratedChain>> {
    if params.joint_count == 0 {
        anyhow::bail!("chain generation needs at least one joint");
    }
    let vertices = collect_target_vertices(input);
    if vertices.is_empty() {
        anyhow::bail!("chain generation found no target vertices");
    }

    let frames = compute_vertex_frames(input, &vertices);
    let chains = split_columns(&frames, params)
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

fn polyline_arc_positions(joints: &[GeneratedJoint], tail: Vector3<f32>) -> (Vec<f32>, f32) {
    let mut arc = vec![0.0];
    let mut current = joints[0].head;
    for joint in &joints[1..] {
        let segment = joint.head - current;
        let length = segment.magnitude();
        current = joint.head;
        arc.push(arc.last().copied().unwrap() + length);
    }
    let last_joint = joints.last().unwrap().head;
    let tail_segment = tail - last_joint;
    let total_length = arc.last().copied().unwrap() + tail_segment.magnitude();
    (arc, total_length)
}

fn project_to_polyline_arc(
    point: Vector3<f32>,
    joints: &[GeneratedJoint],
    tail: Vector3<f32>,
    arc: &[f32],
    total_length: f32,
) -> f32 {
    let nearest = joints
        .iter()
        .enumerate()
        .map(|(i, joint)| {
            let (next_pos, next_arc) = match joints.get(i + 1) {
                Some(next_joint) => (next_joint.head, arc[i + 1]),
                None => (tail, total_length),
            };
            let (t, proj) = line_projection(point, joint.head, next_pos);
            let dist_sq = (point - proj).magnitude2();
            (dist_sq, arc[i] + t * (next_arc - arc[i]))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));

    nearest.map_or(0.0, |(_, s)| s).clamp(0.0, total_length)
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

fn joint_weights_at_arc(s: f32, arc: &[f32], total_length: f32) -> Vec<(usize, f32)> {
    let last_joint_index = arc.len() - 1;

    if s >= total_length {
        return vec![(last_joint_index, 1.0)];
    }

    for (i, &arc_i) in arc.iter().enumerate() {
        if i == last_joint_index {
            return vec![(i, 1.0)];
        }
        let arc_next = arc[i + 1];
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

pub fn compute_chain_weights(
    input: &ChainGenInput,
    params: &ChainGenParams,
    chains: &[GeneratedChain],
) -> Vec<VertexInfluences> {
    let mut result = Vec::new();

    for (chain_index, chain) in chains.iter().enumerate() {
        let (arc, total_length) = polyline_arc_positions(&chain.joints, chain.tail);
        let joint_count = chain.joints.len();

        let root_blend_width = if let Some(w) = params.root_blend_width {
            w
        } else if joint_count == 1 {
            total_length
        } else {
            arc[1] - arc[0]
        };

        for &vertex in &chain.vertices {
            let s = project_to_polyline_arc(
                input.positions[vertex as usize],
                &chain.joints,
                chain.tail,
                &arc,
                total_length,
            );

            let mut influences: Vec<(InfluenceTarget, f32)> =
                joint_weights_at_arc(s, &arc, total_length)
                    .into_iter()
                    .map(|(joint_index, weight)| {
                        (
                            InfluenceTarget::Joint {
                                chain: chain_index,
                                joint: joint_index,
                            },
                            weight,
                        )
                    })
                    .collect();

            if s < root_blend_width {
                let blend_factor = s / root_blend_width;
                let root_weight = 1.0 - blend_factor;
                for influence in &mut influences {
                    influence.1 *= blend_factor;
                }
                influences.push((InfluenceTarget::Root, root_weight));
            }
            influences.retain(|(_, weight)| *weight > 0.0);

            let sum: f32 = influences.iter().map(|(_, w)| w).sum();
            if sum > 1e-8 {
                for influence in &mut influences {
                    influence.1 /= sum;
                }
            }

            result.push(VertexInfluences { vertex, influences });
        }
    }

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

    fn build_hair_params(joint_count: usize) -> ChainGenParams {
        ChainGenParams {
            role: ChainRole::Hair,
            joint_count,
            column_count: 1,
            root_blend_width: None,
        }
    }

    #[test]
    fn straight_strand_places_heads_at_equal_spacing() {
        let (positions, triangles) = build_straight_strand();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            up: Vector3::unit_y(),
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
        let (positions, triangles) = build_straight_strand();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            up: Vector3::unit_y(),
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
            up: Vector3::unit_y(),
        };
        let params = ChainGenParams {
            role: ChainRole::Skirt,
            joint_count: 2,
            column_count: 8,
            root_blend_width: None,
        };

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
            up: Vector3::unit_y(),
        };
        let params = ChainGenParams {
            role: ChainRole::Hair,
            joint_count: 2,
            column_count: 0,
            root_blend_width: None,
        };

        let chains = place_chains(&input, &params).unwrap();
        assert_eq!(chains.len(), 2);
    }

    #[test]
    fn weights_sum_to_one() {
        let (positions, triangles) = build_straight_strand();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            up: Vector3::unit_y(),
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
        let (mut positions, triangles) = build_straight_strand();
        let mut chains = place_chains(
            &ChainGenInput {
                positions: &positions,
                triangles: &triangles,
                root_position: Vector3::new(0.0, 0.0, 0.0),
                up: Vector3::unit_y(),
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
            up: Vector3::unit_y(),
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
        let (positions, triangles) = build_straight_strand();
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            up: Vector3::unit_y(),
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
        let positions: Vec<Vector3<f32>> = (0..6)
            .map(|i| Vector3::new(0.0, 0.2 - 0.1 * i as f32, 0.0))
            .collect();
        let triangles = vec![[0u32, 1, 2], [2, 3, 4], [4, 5, 0]];
        let input = ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            up: Vector3::unit_y(),
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
}
