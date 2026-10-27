use std::f32::consts::TAU;

use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::{fit_principal_axis, resample_polyline, world_y_axis};

use super::chain_gen::{build_phi_basis, progress_of};
use crate::garment::components::chain_gen::{
    joint_name, ChainGenInput, ChainGenParams, GeneratedChain, GeneratedJoint,
    StrandCenterlineRecord, StrandMode,
};

pub const CENTERLINE_SAMPLE_COUNT: usize = 16;
pub const MIN_STRAND_UV_CORRELATION: f32 = 0.8;

pub fn uv_strand_correlation(
    positions: &[Vector3<f32>],
    tex_coords: &[[f32; 2]],
    members: &[u32],
) -> Option<f32> {
    let strand_positions: Vec<Vector3<f32>> =
        members.iter().map(|&v| positions[v as usize]).collect();
    let principal_axis = fit_principal_axis(&strand_positions)?;

    let n = members.len() as f32;
    let projections: Vec<f32> = strand_positions
        .iter()
        .map(|p| (p - principal_axis.mean).dot(principal_axis.axis))
        .collect();
    let mean_projection = projections.iter().sum::<f32>() / n;
    let mean_v = members
        .iter()
        .map(|&v| tex_coords[v as usize][1])
        .sum::<f32>()
        / n;

    let mut sum_xy = 0.0;
    let mut sum_x2 = 0.0;
    let mut sum_y2 = 0.0;
    for (&projection, &member) in projections.iter().zip(members) {
        let x = projection - mean_projection;
        let y = tex_coords[member as usize][1] - mean_v;
        sum_xy += x * y;
        sum_x2 += x * x;
        sum_y2 += y * y;
    }

    if sum_x2 <= 0.0 || sum_y2 <= 0.0 {
        return None;
    }

    Some(sum_xy / (sum_x2 * sum_y2).sqrt())
}

fn build_v_bin_centroids(
    positions: &[Vector3<f32>],
    tex_coords: &[[f32; 2]],
    members: &[u32],
) -> Vec<Vector3<f32>> {
    let v_of = |member: u32| tex_coords[member as usize][1];
    let min_v = members
        .iter()
        .map(|&m| v_of(m))
        .reduce(f32::min)
        .unwrap_or_default();
    let max_v = members
        .iter()
        .map(|&m| v_of(m))
        .reduce(f32::max)
        .unwrap_or_default();
    let bin_width = (max_v - min_v) / CENTERLINE_SAMPLE_COUNT as f32;

    let mut bin_sums = vec![(Vector3::new(0.0, 0.0, 0.0), 0usize); CENTERLINE_SAMPLE_COUNT];
    for &member in members {
        let bin = if bin_width > 0.0 {
            (((v_of(member) - min_v) / bin_width) as usize).min(CENTERLINE_SAMPLE_COUNT - 1)
        } else {
            0
        };
        bin_sums[bin].0 += positions[member as usize];
        bin_sums[bin].1 += 1;
    }

    bin_sums
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(sum, count)| sum / count as f32)
        .collect()
}

fn card_cutoff_progress(
    positions: &[Vector3<f32>],
    members: &[u32],
    root_position: Vector3<f32>,
    root_rigid_length: f32,
) -> Option<f32> {
    let up = world_y_axis();
    members
        .iter()
        .map(|&v| progress_of(positions[v as usize], root_position, up))
        .reduce(f32::min)
        .map(|p| p + root_rigid_length)
}

pub fn strand_centerline(
    positions: &[Vector3<f32>],
    tex_coords: &[[f32; 2]],
    members: &[u32],
    root_position: Vector3<f32>,
    root_rigid_length: f32,
) -> Option<Vec<Vector3<f32>>> {
    let up = world_y_axis();
    let mut centroids = build_v_bin_centroids(positions, tex_coords, members);

    let (&first, &last) = (centroids.first()?, centroids.last()?);
    if progress_of(last, root_position, up) < progress_of(first, root_position, up) {
        centroids.reverse();
    }

    let cutoff = card_cutoff_progress(positions, members, root_position, root_rigid_length)?;
    centroids.retain(|&centroid| progress_of(centroid, root_position, up) >= cutoff);
    let &tip = centroids.last()?;
    if centroids.len() < 2 {
        return None;
    }

    let mut points = resample_polyline(&centroids, CENTERLINE_SAMPLE_COUNT - 1);
    points.push(tip);

    Some(points)
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrandMergeStop {
    Count(usize),
    Distance(f32),
    Never,
}

impl StrandMergeStop {
    pub fn from_settings(column_count: u32, strand_merge_distance: f32) -> Self {
        if column_count > 0 {
            Self::Count(column_count as usize)
        } else if strand_merge_distance > 0.0 {
            Self::Distance(strand_merge_distance)
        } else {
            Self::Never
        }
    }
}

fn centerline_distance(a: &[Vector3<f32>], b: &[Vector3<f32>]) -> f32 {
    let len = a.len().min(b.len());
    a.iter()
        .zip(b)
        .map(|(p, q)| (p - q).magnitude())
        .sum::<f32>()
        / len as f32
}

pub fn merge_strands(centerlines: &[Vec<Vector3<f32>>], stop: StrandMergeStop) -> Vec<Vec<usize>> {
    let n = centerlines.len();
    if n == 0 {
        return vec![];
    }

    let mut clusters: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();

    loop {
        let Some((i, j, distance)) = closest_cluster_pair(&clusters, centerlines) else {
            break;
        };

        let reached_stop = match stop {
            StrandMergeStop::Never => true,
            StrandMergeStop::Count(target) => clusters.len() <= target,
            StrandMergeStop::Distance(threshold) => distance > threshold,
        };
        if reached_stop {
            break;
        }

        let mut merged = clusters[i].clone();
        merged.append(&mut clusters[j]);
        clusters.remove(j);
        clusters[i] = merged;
    }

    for cluster in &mut clusters {
        cluster.sort_unstable();
    }
    clusters.sort_by_key(|c| c[0]);

    clusters
}

fn closest_cluster_pair(
    clusters: &[Vec<usize>],
    centerlines: &[Vec<Vector3<f32>>],
) -> Option<(usize, usize, f32)> {
    if clusters.len() < 2 {
        return None;
    }

    let (mut best_i, mut best_j) = (0, 1);
    let mut best_dist = cluster_distance(&clusters[0], &clusters[1], centerlines);

    for i in 0..clusters.len() {
        for j in (i + 1)..clusters.len() {
            let dist = cluster_distance(&clusters[i], &clusters[j], centerlines);
            if dist < best_dist {
                best_dist = dist;
                best_i = i;
                best_j = j;
            }
        }
    }

    Some((best_i, best_j, best_dist))
}

fn cluster_distance(a: &[usize], b: &[usize], centerlines: &[Vec<Vector3<f32>>]) -> f32 {
    let mut total = 0.0;
    let mut count = 0usize;
    for &ai in a {
        for &bi in b {
            total += centerline_distance(&centerlines[ai], &centerlines[bi]);
            count += 1;
        }
    }
    total / count as f32
}

pub fn tuft_centerline(
    centerlines: &[Vec<Vector3<f32>>],
    weights: &[usize],
    members: &[usize],
) -> Vec<Vector3<f32>> {
    let total_weight: usize = members.iter().map(|&c| weights[c]).sum();
    if total_weight == 0 || centerlines.is_empty() {
        return vec![];
    }

    let sample_count = centerlines[members[0]].len();
    let mut result = Vec::with_capacity(sample_count);

    for k in 0..sample_count {
        let mut sum = Vector3::new(0.0, 0.0, 0.0);
        for &c in members {
            sum += centerlines[c][k] * weights[c] as f32;
        }
        result.push(sum / total_weight as f32);
    }

    result
}

pub fn decide_strand_mode(
    input: &ChainGenInput,
    components: &[Vec<u32>],
    band_progress: f32,
) -> StrandMode {
    if input.tex_coords.is_empty() {
        return StrandMode::Azimuth;
    }

    let up = world_y_axis();
    let mut total = 0;
    let mut correlated = 0;
    for members in components {
        let is_correlated = uv_strand_correlation(input.positions, input.tex_coords, members)
            .is_some_and(|r| r.abs() >= MIN_STRAND_UV_CORRELATION);
        if is_correlated {
            total += members.len();
            correlated += members.len();
        } else {
            let below_count = members
                .iter()
                .filter(|&&v| {
                    progress_of(input.positions[v as usize], input.root_position, up)
                        > band_progress
                })
                .count();
            total += below_count;
        }
    }

    if correlated * 2 > total {
        StrandMode::Card
    } else {
        StrandMode::Azimuth
    }
}

struct CardStrand {
    component: u32,
    centerline: Vec<Vector3<f32>>,
    below_band: Vec<u32>,
}

impl CardStrand {
    fn centerline_record(&self, chain: usize) -> StrandCenterlineRecord {
        StrandCenterlineRecord {
            component: self.component,
            chain,
            points: self.centerline.iter().map(|p| [p.x, p.y, p.z]).collect(),
        }
    }
}

fn build_card_strand(
    input: &ChainGenInput,
    members: &[u32],
    root_rigid_length: f32,
) -> Option<CardStrand> {
    let component = members.iter().copied().min()?;
    let correlation = uv_strand_correlation(input.positions, input.tex_coords, members)?;
    if correlation.abs() < MIN_STRAND_UV_CORRELATION {
        return None;
    }
    let centerline = strand_centerline(
        input.positions,
        input.tex_coords,
        members,
        input.root_position,
        root_rigid_length,
    )?;

    let up = world_y_axis();
    let cutoff = card_cutoff_progress(
        input.positions,
        members,
        input.root_position,
        root_rigid_length,
    )?;
    let below_band = members
        .iter()
        .copied()
        .filter(|&v| progress_of(input.positions[v as usize], input.root_position, up) >= cutoff)
        .collect();

    Some(CardStrand {
        component,
        centerline,
        below_band,
    })
}

fn build_tuft_chain(
    column: usize,
    strands: &[CardStrand],
    tuft: &[usize],
    centerline: Vec<Vector3<f32>>,
    params: &ChainGenParams,
) -> GeneratedChain {
    let joints: Vec<GeneratedJoint> =
        resample_polyline(&centerline, params.settings.joint_count as usize)
            .into_iter()
            .enumerate()
            .map(|(joint, head)| GeneratedJoint {
                name: joint_name(params, column, joint),
                head,
            })
            .collect();
    let mut vertices: Vec<u32> = tuft
        .iter()
        .flat_map(|&strand| strands[strand].below_band.iter().copied())
        .collect();
    vertices.sort_unstable();

    let up = world_y_axis();
    let tail = match tuft
        .iter()
        .filter_map(|&strand| strands[strand].centerline.last().copied())
        .min_by(|a, b| a.dot(up).total_cmp(&b.dot(up)))
    {
        Some(t) => t,
        None => centerline[centerline.len() - 1],
    };

    GeneratedChain {
        column,
        joints,
        tail,
        vertices,
    }
}

pub(crate) fn place_card_chains(
    input: &ChainGenInput,
    components: &[Vec<u32>],
    phi_center: Vector3<f32>,
    params: &ChainGenParams,
) -> (Vec<GeneratedChain>, Vec<StrandCenterlineRecord>) {
    let strands: Vec<CardStrand> = components
        .iter()
        .filter_map(|members| {
            build_card_strand(input, members, params.settings.card.root_rigid_length)
        })
        .collect();
    let centerlines: Vec<Vec<Vector3<f32>>> = strands
        .iter()
        .map(|strand| strand.centerline.clone())
        .collect();
    let weights: Vec<usize> = strands
        .iter()
        .map(|strand| strand.below_band.len())
        .collect();
    let stop = StrandMergeStop::from_settings(
        params.settings.column_count,
        params.settings.strand_merge_distance,
    );
    let tufts = merge_strands(&centerlines, stop);

    let (reference, side) = build_phi_basis(world_y_axis());
    let mut tuft_centerlines: Vec<(f32, Vec<usize>, Vec<Vector3<f32>>)> = tufts
        .into_iter()
        .map(|tuft| {
            let centerline = tuft_centerline(&centerlines, &weights, &tuft);
            let offset = centerline[0] - phi_center;
            let phi_around_up = offset
                .dot(side)
                .atan2(offset.dot(reference))
                .rem_euclid(TAU);
            (phi_around_up, tuft, centerline)
        })
        .collect();
    tuft_centerlines.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut records: Vec<StrandCenterlineRecord> = Vec::new();
    let chains = tuft_centerlines
        .into_iter()
        .enumerate()
        .map(|(column, (_, tuft, centerline))| {
            records.extend(tuft.iter().map(|&s| strands[s].centerline_record(column)));
            build_tuft_chain(column, &strands, &tuft, centerline, params)
        })
        .collect();
    records.sort_by_key(|record| record.component);
    (chains, records)
}

#[cfg(test)]
mod tests;
