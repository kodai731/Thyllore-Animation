use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::{resample_polyline, tangent_frame_around, world_y_axis, TAU};

use super::frame::progress_of;
use super::strand_merge::{merge_strands, tuft_centerline, StrandMergeStop};
use super::strand_mode::{uv_strand_correlation, MIN_STRAND_UV_CORRELATION};
use crate::garment::components::chain_gen::{
    joint_name, ChainGenInput, ChainGenParams, GeneratedChain, GeneratedJoint,
    StrandCenterlineRecord,
};

pub const CENTERLINE_SAMPLE_COUNT: usize = 16;
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

    let (reference, side) = tangent_frame_around(world_y_axis());
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
