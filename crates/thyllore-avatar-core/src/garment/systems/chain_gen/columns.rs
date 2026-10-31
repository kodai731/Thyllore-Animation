use std::collections::{BTreeMap, BTreeSet};

use thyllore_math_core::{
    angle_bin, circular_mean_and_concentration, cluster_angles, count_circular_modes,
    smoothed_circular_histogram, triangle_component_labels, WeightedAngle, PI,
};

use super::frame::VertexFrame;
use super::root_band::RootBand;
use crate::garment::components::chain_gen::{ChainGenInput, ChainGenParams};

const AZIMUTH_BIN_COUNT: usize = 36;
const KMEANS_ITERATION_COUNT: usize = 16;
const MIN_STRAND_AZIMUTH_CONCENTRATION: f32 = 0.5;

fn unit_sample(phi: f32) -> WeightedAngle {
    WeightedAngle {
        angle: phi,
        weight: 1.0,
    }
}

/// The column count setting, or the number of φ modes when the setting is 0.
fn resolve_column_count(params: &ChainGenParams, samples: &[WeightedAngle]) -> usize {
    match params.settings.column_count as usize {
        0 => count_circular_modes(&smoothed_circular_histogram(samples, AZIMUTH_BIN_COUNT)),
        count => count,
    }
}

fn group_into_columns(
    assigned: impl Iterator<Item = (usize, Vec<usize>)>,
    column_count: usize,
) -> Vec<Vec<usize>> {
    let mut columns = vec![Vec::new(); column_count];
    for (column, frame_indices) in assigned {
        columns[column].extend(frame_indices);
    }
    columns.retain(|members| !members.is_empty());
    columns
}

pub(super) fn split_garment_columns(
    frames: &[VertexFrame],
    params: &ChainGenParams,
) -> Vec<Vec<usize>> {
    let samples: Vec<WeightedAngle> = frames
        .iter()
        .map(|frame| unit_sample(frame.phi_around_up))
        .collect();
    let column_count = resolve_column_count(params, &samples);
    let assigned = frames.iter().enumerate().map(|(frame_index, frame)| {
        let column = angle_bin(frame.phi_around_up + PI / column_count as f32, column_count);
        (column, vec![frame_index])
    });
    group_into_columns(assigned, column_count)
}

/// Connected component label per vertex: the smallest vertex index of its component.
pub fn component_labels(input: &ChainGenInput) -> Vec<usize> {
    triangle_component_labels(input.positions.len(), input.triangles)
}

pub(super) fn group_into_components(input: &ChainGenInput, vertices: &[u32]) -> Vec<Vec<u32>> {
    let labels = component_labels(input);
    let mut components: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
    for &vertex in vertices {
        components
            .entry(labels[vertex as usize])
            .or_default()
            .push(vertex);
    }
    components.into_values().collect()
}

/// Strands below the root band, one column per φ cluster; a connected component never splits,
/// and a component spread around the head (the scalp shell) stays on the root.
pub(super) fn split_hair_columns(
    input: &ChainGenInput,
    frames: &[VertexFrame],
    band: RootBand,
    params: &ChainGenParams,
) -> Vec<Vec<usize>> {
    let strand_frames: Vec<usize> = (0..frames.len())
        .filter(|&index| frames[index].progress > band.progress)
        .collect();
    let strand_samples: Vec<WeightedAngle> = strand_frames
        .iter()
        .map(|&index| unit_sample(frames[index].phi_around_up))
        .collect();
    let column_count = resolve_column_count(params, &strand_samples);

    let labels = component_labels(input);
    let component_count = frames
        .iter()
        .map(|frame| labels[frame.vertex as usize])
        .collect::<BTreeSet<_>>()
        .len();
    if component_count < 2 {
        let columns = cluster_angles(
            &strand_samples,
            column_count,
            AZIMUTH_BIN_COUNT,
            KMEANS_ITERATION_COUNT,
        );
        let assigned = columns.into_iter().zip(
            strand_frames
                .into_iter()
                .map(|frame_index| vec![frame_index]),
        );
        return group_into_columns(assigned, column_count);
    }

    let strands = one_sided_strands(&strand_frames, frames, &labels);
    let strand_phis: Vec<WeightedAngle> = strands
        .iter()
        .map(|(phi, members)| WeightedAngle {
            angle: *phi,
            weight: members.len() as f32,
        })
        .collect();
    let columns = cluster_angles(
        &strand_phis,
        column_count,
        AZIMUTH_BIN_COUNT,
        KMEANS_ITERATION_COUNT,
    );
    let assigned = columns
        .into_iter()
        .zip(strands.into_iter().map(|(_, members)| members));
    group_into_columns(assigned, column_count)
}

/// Strand frames grouped by connected component, with each component's mean φ; a component
/// wrapped around the head is dropped.
fn one_sided_strands(
    strand_frames: &[usize],
    frames: &[VertexFrame],
    labels: &[usize],
) -> Vec<(f32, Vec<usize>)> {
    let mut by_component: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &frame_index in strand_frames {
        by_component
            .entry(labels[frames[frame_index].vertex as usize])
            .or_default()
            .push(frame_index);
    }

    by_component
        .into_values()
        .filter_map(|members| {
            let phis = members
                .iter()
                .map(|&i| unit_sample(frames[i].phi_around_up));
            let (mean, concentration) = circular_mean_and_concentration(phis)?;
            (concentration >= MIN_STRAND_AZIMUTH_CONCENTRATION).then_some((mean, members))
        })
        .collect()
}
