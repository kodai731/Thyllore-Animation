use std::collections::{BTreeMap, BTreeSet};

use thyllore_math_core::TriangleBvh;
use thyllore_model_core::BoneId;

use super::frame::{BandPartition, VertexFrame};
use crate::cloth_setup::components::chain_gen::{
    ChainGenInput, ChainGenParams, ChainRole, CoveredVertex, SkinSurface,
};

/// The target vertices the skin covers, each with the influences of its nearest skin vertex.
pub(super) struct GarmentCoverage {
    covered: BTreeMap<u32, Vec<(BoneId, f32)>>,
    source: CoverageSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoverageSource {
    /// Measured by distance to the skin; the chain starts at the first band under the covered share
    Measured,
    /// Marked by the user; the chain is every vertex outside the set
    Marked,
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

struct SkinSurfaces<'a> {
    surfaces: Vec<(&'a SkinSurface, TriangleBvh)>,
}

impl<'a> SkinSurfaces<'a> {
    fn build(skins: &'a [SkinSurface]) -> Self {
        let surfaces = skins
            .iter()
            .filter(|skin| !skin.triangles.is_empty())
            .map(|skin| (skin, TriangleBvh::build(&skin.positions, &skin.triangles)))
            .collect();
        Self { surfaces }
    }

    fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
    }

    /// The influences of the skin vertex nearest to `point` within `max_distance`.
    fn nearest_influences(
        &self,
        point: cgmath::Vector3<f32>,
        max_distance: f32,
    ) -> Option<Vec<(BoneId, f32)>> {
        let (_, corner, skin) = self
            .surfaces
            .iter()
            .filter_map(|(skin, bvh)| {
                let hit = bvh.closest_point(point, max_distance)?;
                Some((
                    hit.distance,
                    bvh.nearest_corner(hit.triangle, hit.point),
                    *skin,
                ))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))?;
        Some(skin.influences[corner as usize].clone())
    }
}

/// The coverage of a garment: the vertices the user marked, else the vertices within `cover_distance`
/// of a skin surface; none for hair or when neither applies.
pub(super) fn garment_coverage(
    input: &ChainGenInput,
    params: &ChainGenParams,
    vertices: &[u32],
) -> Option<GarmentCoverage> {
    if params.role != ChainRole::Garment {
        return None;
    }
    let surfaces = SkinSurfaces::build(input.skins);
    if !input.covered_vertices.is_empty() {
        return Some(marked_coverage(input, &surfaces, vertices));
    }

    let cover_distance = params.settings.garment.cover_distance;
    if cover_distance <= 0.0 || surfaces.is_empty() {
        return None;
    }
    let covered = vertices
        .iter()
        .filter_map(|&vertex| {
            let influences =
                surfaces.nearest_influences(input.positions[vertex as usize], cover_distance)?;
            Some((vertex, influences))
        })
        .collect();
    Some(GarmentCoverage {
        covered,
        source: CoverageSource::Measured,
    })
}

/// The marked vertices with the influences of their nearest skin vertex, or the root bone without a surface.
fn marked_coverage(
    input: &ChainGenInput,
    surfaces: &SkinSurfaces,
    vertices: &[u32],
) -> GarmentCoverage {
    let marked: BTreeSet<u32> = input.covered_vertices.iter().copied().collect();
    let covered = vertices
        .iter()
        .filter(|vertex| marked.contains(vertex))
        .map(|&vertex| {
            let influences = surfaces
                .nearest_influences(input.positions[vertex as usize], f32::MAX)
                .unwrap_or_else(|| vec![(input.root_bone, 1.0)]);
            (vertex, influences)
        })
        .collect();
    GarmentCoverage {
        covered,
        source: CoverageSource::Marked,
    }
}

/// The frames the chain is built from: every uncovered frame for a marked coverage, the uncovered
/// frames below the covered bands for a measured one. Every frame left out joins the coverage (the root
/// bone drives the ones the skin does not reach), so marking the coverage again reproduces the
/// chain. Fails when nothing is left for the chain.
pub(super) fn chain_frames(
    frames: Vec<VertexFrame>,
    coverage: &mut GarmentCoverage,
    input: &ChainGenInput,
    params: &ChainGenParams,
) -> anyhow::Result<Vec<VertexFrame>> {
    let remaining = match coverage.source {
        CoverageSource::Marked => frames
            .iter()
            .filter(|frame| !coverage.is_covered(frame.vertex))
            .copied()
            .collect(),
        CoverageSource::Measured => frames_below_coverage(&frames, coverage, params)?,
    };
    if remaining.is_empty() {
        anyhow::bail!("the skin covers every vertex of the garment; no chain is needed");
    }

    let chained: BTreeSet<u32> = remaining.iter().map(|frame| frame.vertex).collect();
    for frame in frames {
        if !chained.contains(&frame.vertex) {
            coverage
                .covered
                .entry(frame.vertex)
                .or_insert_with(|| vec![(input.root_bone, 1.0)]);
        }
    }
    Ok(remaining)
}

fn frames_below_coverage(
    frames: &[VertexFrame],
    coverage: &GarmentCoverage,
    params: &ChainGenParams,
) -> anyhow::Result<Vec<VertexFrame>> {
    let bands = BandPartition::of_frames(frames, params.settings.joint_count as usize);
    let mut counts = vec![(0usize, 0usize); bands.count];
    for frame in frames {
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
        .iter()
        .filter(|frame| frame.progress >= start_progress && !coverage.is_covered(frame.vertex))
        .copied()
        .collect())
}
