use std::collections::BTreeMap;

use thyllore_math_core::TriangleBvh;
use thyllore_model_core::BoneId;

use super::frame::{BandPartition, VertexFrame};
use crate::cloth_setup::components::chain_gen::{
    ChainGenInput, ChainGenParams, ChainRole, CoveredVertex,
};

/// The target vertices the skin covers, each with the influences of its nearest skin vertex.
pub(super) struct GarmentCoverage {
    covered: BTreeMap<u32, Vec<(BoneId, f32)>>,
}

impl GarmentCoverage {
    pub(super) fn is_covered(&self, vertex: u32) -> bool {
        self.covered.contains_key(&vertex)
    }

    pub(super) fn into_covered_vertices(self) -> Vec<CoveredVertex> {
        self.covered
            .into_iter()
            .map(|(vertex, influences)| CoveredVertex { vertex, influences })
            .collect()
    }
}

/// The coverage of a garment by the skins within `cover_distance`; none for hair or without a skin surface.
pub(super) fn garment_coverage(
    input: &ChainGenInput,
    params: &ChainGenParams,
    vertices: &[u32],
) -> Option<GarmentCoverage> {
    let cover_distance = params.settings.garment.cover_distance;
    let has_surface = input.skins.iter().any(|skin| !skin.triangles.is_empty());
    if params.role != ChainRole::Garment || cover_distance <= 0.0 || !has_surface {
        return None;
    }

    let surfaces: Vec<(&_, TriangleBvh)> = input
        .skins
        .iter()
        .filter(|skin| !skin.triangles.is_empty())
        .map(|skin| (skin, TriangleBvh::build(&skin.positions, &skin.triangles)))
        .collect();
    let covered = vertices
        .iter()
        .filter_map(|&vertex| {
            let point = input.positions[vertex as usize];
            let nearest = surfaces
                .iter()
                .filter_map(|(skin, bvh)| {
                    let hit = bvh.closest_point(point, cover_distance)?;
                    Some((
                        hit.distance,
                        bvh.nearest_corner(hit.triangle, hit.point),
                        *skin,
                    ))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))?;
            let (_, corner, skin) = nearest;
            Some((vertex, skin.influences[corner as usize].clone()))
        })
        .collect();
    Some(GarmentCoverage { covered })
}

/// The frames below the covered bands: the chain starts at the first band whose covered share is
/// under `covered_band_ratio`. Fails when every band is covered.
pub(super) fn frames_below_coverage(
    frames: Vec<VertexFrame>,
    coverage: &GarmentCoverage,
    params: &ChainGenParams,
) -> anyhow::Result<Vec<VertexFrame>> {
    let bands = BandPartition::of_frames(&frames, params.settings.joint_count as usize);
    let mut counts = vec![(0usize, 0usize); bands.count];
    for frame in &frames {
        let band = bands.band_of(frame.progress);
        counts[band].0 += usize::from(coverage.is_covered(frame.vertex));
        counts[band].1 += 1;
    }

    let ratio = params.settings.garment.covered_band_ratio;
    let start = counts
        .iter()
        .position(|&(covered, total)| total == 0 || (covered as f32) < ratio * total as f32)
        .ok_or_else(|| {
            anyhow::anyhow!("the skin covers every band of the garment; no chain is needed")
        })?;
    let start_progress = bands.start_of(start);
    Ok(frames
        .into_iter()
        .filter(|frame| frame.progress >= start_progress)
        .collect())
}
