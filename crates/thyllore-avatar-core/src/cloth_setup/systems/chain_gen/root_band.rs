use cgmath::Vector3;
use thyllore_math_core::{centroid, widest_cross_section, world_y_axis, CROSS_SECTION_SLICE_COUNT};

use super::frame::{attachment_or_target, collect_target_vertices};
use crate::cloth_setup::components::chain_gen::{ChainGenInput, ChainGenParams, ChainRole};

/// The hair root band in root-relative progress (distance down the up axis from the root bone).
#[derive(Debug, Clone, Copy)]
pub struct RootBand {
    pub equator_progress: f32,
    pub progress: f32,
    pub center: Vector3<f32>,
}

/// The widest horizontal cross-section of the attachment surface, shifted by the setting.
fn estimate_root_band(
    attachment: &[Vector3<f32>],
    root_position: Vector3<f32>,
    offset: f32,
) -> RootBand {
    let (members, equator_progress) = widest_cross_section(
        attachment,
        root_position,
        -world_y_axis(),
        CROSS_SECTION_SLICE_COUNT,
    )
    .map_or((Vec::new(), 0.0), |slice| (slice.members, slice.distance));
    RootBand {
        equator_progress,
        progress: equator_progress + offset,
        center: centroid(&members).unwrap_or(Vector3::new(0.0, 0.0, 0.0)),
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
