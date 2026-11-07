use std::collections::{BTreeMap, BTreeSet};

use thyllore_math_core::world_y_axis;
use thyllore_model_core::BoneId;

use super::frame::VertexFrame;
use super::support::{garment_support, nearest_body_influences};
use crate::cloth_setup::components::chain_gen::{
    ChainGenInput, ChainGenParams, ChainRole, CoveredVertex, GarmentSupport,
};

/// The target vertices the body covers, each with the influences of its nearest body point.
pub(super) struct GarmentCoverage {
    covered: BTreeMap<u32, Vec<(BoneId, f32)>>,
    source: CoverageSource,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CoverageSource {
    /// The body supports the garment down to a ring; the chain is every vertex below it
    Supported(GarmentSupport),
    /// Marked by the user; the chain is every vertex outside the set
    Marked,
}

impl GarmentCoverage {
    pub(super) fn is_covered(&self, vertex: u32) -> bool {
        self.covered.contains_key(&vertex)
    }

    pub(super) fn support(&self) -> Option<GarmentSupport> {
        match self.source {
            CoverageSource::Supported(support) => Some(support),
            CoverageSource::Marked => None,
        }
    }

    pub(super) fn into_covered_vertices(self) -> Vec<CoveredVertex> {
        self.covered
            .into_iter()
            .map(|(vertex, influences)| CoveredVertex { vertex, influences })
            .collect()
    }
}

/// The coverage of a garment: the vertices the user marked, else the vertices at or above the ring
/// the body supports; none for hair, without a body, or when the body supports nothing.
pub(super) fn garment_coverage(
    input: &ChainGenInput,
    params: &ChainGenParams,
    vertices: &[u32],
) -> anyhow::Result<Option<GarmentCoverage>> {
    if let Some(marked) = marked_garment_coverage(input, params, vertices) {
        return Ok(Some(marked));
    }
    if params.role != ChainRole::Garment {
        return Ok(None);
    }
    let Some(support) = garment_support(input, params, vertices)? else {
        return Ok(None);
    };
    let up = world_y_axis();
    let covered = vertices
        .iter()
        .filter(|&&vertex| input.positions[vertex as usize].dot(up) >= support.boundary_height)
        .map(|&vertex| (vertex, body_influences(input, vertex)))
        .collect();
    Ok(Some(GarmentCoverage {
        covered,
        source: CoverageSource::Supported(support),
    }))
}

fn body_influences(input: &ChainGenInput, vertex: u32) -> Vec<(BoneId, f32)> {
    nearest_body_influences(input.skins, input.positions[vertex as usize])
        .unwrap_or_else(|| vec![(input.root_bone, 1.0)])
}

/// The vertices the user marked as covered, with the influences of their nearest body point, or the root
/// bone without a body; none for hair or when nothing is marked.
pub(super) fn marked_garment_coverage(
    input: &ChainGenInput,
    params: &ChainGenParams,
    vertices: &[u32],
) -> Option<GarmentCoverage> {
    if params.role != ChainRole::Garment || input.covered_vertices.is_empty() {
        return None;
    }
    let marked: BTreeSet<u32> = input.covered_vertices.iter().copied().collect();
    let covered = vertices
        .iter()
        .filter(|vertex| marked.contains(vertex))
        .map(|&vertex| (vertex, body_influences(input, vertex)))
        .collect();
    Some(GarmentCoverage {
        covered,
        source: CoverageSource::Marked,
    })
}

/// The frames the chain is built from: every uncovered frame. Fails when nothing is left for the chain.
pub(super) fn chain_frames(
    frames: Vec<VertexFrame>,
    coverage: &GarmentCoverage,
) -> anyhow::Result<Vec<VertexFrame>> {
    let remaining: Vec<VertexFrame> = frames
        .into_iter()
        .filter(|frame| !coverage.is_covered(frame.vertex))
        .collect();
    if remaining.is_empty() {
        anyhow::bail!("the body covers every vertex of the garment; no chain is needed");
    }
    Ok(remaining)
}
