use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::{fit_principal_axis, world_y_axis};

use super::frame::progress_of;
use crate::cloth_setup::components::chain_gen::{ChainGenInput, StrandMode};

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
