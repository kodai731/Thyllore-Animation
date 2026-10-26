use cgmath::Vector3;
use thyllore_math_core::resample_polyline;

use super::columns::{group_into_components, split_garment_columns, split_hair_columns};
use super::frame::{collect_target_vertices, compute_vertex_frames, VertexFrame};
use super::root_band::{hair_root_band, RootBand};
use crate::garment::components::chain_gen::{
    joint_name, ChainGenInput, ChainGenParams, ChainPlacement, GeneratedChain, GeneratedJoint,
    StrandMode, StrandPlacement,
};
use crate::garment::systems::card_strands::{decide_strand_mode, place_card_chains};

/// Band centroids from the root down plus the deepest vertex as the tail.
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
            name: joint_name(params, column, joint),
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

fn place_hair_card_chains(
    input: &ChainGenInput,
    vertices: &[u32],
    band: RootBand,
    params: &ChainGenParams,
) -> Option<ChainPlacement> {
    let components = group_into_components(input, vertices);
    match decide_strand_mode(input, &components, band.progress) {
        StrandMode::Card => {
            let (chains, centerlines) = place_card_chains(input, &components, band.center, params);
            if chains.is_empty() {
                return None;
            }
            Some(ChainPlacement {
                chains,
                strands: Some(StrandPlacement {
                    mode: StrandMode::Card,
                    centerlines,
                }),
            })
        }
        StrandMode::Azimuth => None,
    }
}

pub fn place_chains(
    input: &ChainGenInput,
    params: &ChainGenParams,
) -> anyhow::Result<Vec<GeneratedChain>> {
    place_chains_with_strands(input, params).map(|placement| placement.chains)
}

pub fn place_chains_with_strands(
    input: &ChainGenInput,
    params: &ChainGenParams,
) -> anyhow::Result<ChainPlacement> {
    if params.settings.joint_count == 0 {
        anyhow::bail!("chain generation needs at least one joint");
    }
    let vertices = collect_target_vertices(input);
    if vertices.is_empty() {
        anyhow::bail!("chain generation found no target vertices");
    }

    let root_band = hair_root_band(input, params);
    let phi_center = root_band.map_or(input.root_position, |band| band.center);
    let frames = compute_vertex_frames(input, &vertices, phi_center);
    let (columns, strands) = match root_band {
        Some(band) => {
            if let Some(card_placement) = place_hair_card_chains(input, &vertices, band, params) {
                return Ok(card_placement);
            }
            let strands = StrandPlacement {
                mode: StrandMode::Azimuth,
                centerlines: Vec::new(),
            };
            (
                split_hair_columns(input, &frames, band, params),
                Some(strands),
            )
        }
        None => (split_garment_columns(&frames, params), None),
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
    Ok(ChainPlacement { chains, strands })
}
