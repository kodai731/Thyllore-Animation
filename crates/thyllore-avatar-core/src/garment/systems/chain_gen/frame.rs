use std::borrow::Cow;
use std::collections::BTreeSet;

use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::world_y_axis;

use crate::garment::components::chain_gen::ChainGenInput;

#[derive(Debug, Clone, Copy)]
pub(super) struct VertexFrame {
    pub vertex: u32,
    /// Angle about the world up axis through the azimuth center, from the reference axis of `build_azimuth_basis`.
    pub azimuth: f32,
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

/// The reference axis azimuth 0 points along and the side axis at azimuth +90°, both orthogonal to `up`.
pub(crate) fn build_azimuth_basis(up: Vector3<f32>) -> (Vector3<f32>, Vector3<f32>) {
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

pub(super) fn compute_vertex_frames(
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

/// The attachment surface when the input has one, otherwise the target vertices themselves.
pub(crate) fn attachment_or_target<'a>(
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
