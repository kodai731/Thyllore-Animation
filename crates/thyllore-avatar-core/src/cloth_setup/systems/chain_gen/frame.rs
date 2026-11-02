use std::collections::BTreeSet;

use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::{tangent_frame_around, world_y_axis};

use crate::cloth_setup::components::chain_gen::ChainGenInput;

#[derive(Debug, Clone, Copy)]
pub(super) struct VertexFrame {
    pub vertex: u32,
    /// Azimuth φ about the world up axis through `phi_center`, from the reference axis of `tangent_frame_around(up)`.
    pub phi_around_up: f32,
    /// Distance down the world up axis from the root bone.
    pub progress: f32,
}

pub(crate) fn collect_target_vertices(input: &ChainGenInput) -> Vec<u32> {
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

pub(crate) fn progress_of(
    position: Vector3<f32>,
    root_position: Vector3<f32>,
    up: Vector3<f32>,
) -> f32 {
    -(position - root_position).dot(up)
}

pub(super) fn compute_vertex_frames(
    input: &ChainGenInput,
    vertices: &[u32],
    phi_center: Vector3<f32>,
) -> Vec<VertexFrame> {
    let up = world_y_axis();
    let (reference, side) = tangent_frame_around(up);

    vertices
        .iter()
        .map(|&vertex| {
            let position = input.positions[vertex as usize];
            let offset = position - phi_center;
            VertexFrame {
                vertex,
                phi_around_up: offset.dot(side).atan2(offset.dot(reference)),
                progress: progress_of(position, input.root_position, up),
            }
        })
        .collect()
}

/// Equal progress slices from the shallowest to the deepest frame, one per joint.
#[derive(Debug, Clone, Copy)]
pub(super) struct BandPartition {
    pub min_progress: f32,
    pub band_width: f32,
    pub count: usize,
}

impl BandPartition {
    pub(super) fn of_frames(frames: &[VertexFrame], count: usize) -> Self {
        let progresses = frames.iter().map(|frame| frame.progress);
        let min_progress = progresses.clone().reduce(f32::min).unwrap_or_default();
        let max_progress = progresses.reduce(f32::max).unwrap_or_default();
        Self {
            min_progress,
            band_width: (max_progress - min_progress) / count as f32,
            count,
        }
    }

    pub(super) fn band_of(&self, progress: f32) -> usize {
        if self.band_width <= 0.0 {
            return 0;
        }
        (((progress - self.min_progress) / self.band_width) as usize).min(self.count - 1)
    }

    pub(super) fn start_of(&self, band: usize) -> f32 {
        self.min_progress + self.band_width * band as f32
    }
}
