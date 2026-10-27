use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::{centroid, world_y_axis};

use super::frame::{attachment_or_target, collect_target_vertices, progress_of};
use crate::garment::components::chain_gen::{ChainGenInput, ChainGenParams, ChainRole};

const ROOT_BAND_SLICE_COUNT: usize = 64;

/// The hair root band in root-relative progress (distance down the up axis from the root bone).
#[derive(Debug, Clone, Copy)]
pub struct RootBand {
    pub equator_progress: f32,
    pub progress: f32,
    pub center: Vector3<f32>,
}

pub(crate) struct WidestSlice {
    pub members: Vec<Vector3<f32>>,
    pub progress: f32,
}

fn horizontal_radius(offset: Vector3<f32>, up: Vector3<f32>) -> f32 {
    (offset - up * offset.dot(up)).magnitude()
}

/// The slice of the attachment surface with the largest horizontal radius about its mean.
pub(crate) fn widest_slice(
    attachment: &[Vector3<f32>],
    root_position: Vector3<f32>,
) -> Option<WidestSlice> {
    let center = centroid(attachment)?;
    let up = world_y_axis();
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
    let widest = (0..ROOT_BAND_SLICE_COUNT)
        .max_by(|a, b| widest_radius[*a].total_cmp(&widest_radius[*b]))
        .unwrap_or(0);

    let members: Vec<Vector3<f32>> = attachment
        .iter()
        .zip(&progresses)
        .filter(|(_, &progress)| slice_of(progress) == widest)
        .map(|(&p, _)| p)
        .collect();
    Some(WidestSlice {
        members,
        progress: min_progress + (widest as f32 + 0.5) * slice_width,
    })
}

/// The widest horizontal cross-section of the attachment surface, shifted by the setting.
fn estimate_root_band(
    attachment: &[Vector3<f32>],
    root_position: Vector3<f32>,
    offset: f32,
) -> RootBand {
    let slice = widest_slice(attachment, root_position).unwrap_or(WidestSlice {
        members: Vec::new(),
        progress: 0.0,
    });
    RootBand {
        equator_progress: slice.progress,
        progress: slice.progress + offset,
        center: centroid(&slice.members).unwrap_or(Vector3::new(0.0, 0.0, 0.0)),
    }
}

/// The root band a hair generation uses; a garment has none.
pub fn hair_root_band(input: &ChainGenInput, params: &ChainGenParams) -> Option<RootBand> {
    match params.role {
        ChainRole::Hair => {
            let vertices = collect_target_vertices(input);
            Some(estimate_root_band(
                &attachment_or_target(input, &vertices),
                input.root_position,
                params.settings.azimuth.root_band_offset,
            ))
        }
        ChainRole::Garment => None,
    }
}
