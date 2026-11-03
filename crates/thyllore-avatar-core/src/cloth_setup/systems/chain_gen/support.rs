use std::collections::BTreeMap;

use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::{centroid, tangent_frame_around, world_y_axis};
use thyllore_model_core::BoneId;

use crate::cloth_setup::components::chain_gen::{
    ChainGenInput, ChainGenParams, GarmentSupport, SkinSurface, SUPPORT_RING_HEIGHT,
};

const RING_HEIGHT: f32 = SUPPORT_RING_HEIGHT;
const SECTOR_COUNT: usize = 12;
const SHELF_REACH_RINGS: usize = 2;

/// Horizontal rings of equal height from `bottom` upward, with azimuth sectors around `axis_center`.
struct RingGrid {
    bottom: f32,
    ring_count: usize,
    axis_center: Vector3<f32>,
    reference: Vector3<f32>,
    side: Vector3<f32>,
}

impl RingGrid {
    fn new(bottom: f32, top: f32, axis_center: Vector3<f32>) -> Self {
        let (reference, side) = tangent_frame_around(world_y_axis());
        Self {
            bottom,
            ring_count: ((top - bottom) / RING_HEIGHT).floor() as usize + 1,
            axis_center,
            reference,
            side,
        }
    }

    fn ring_of(&self, height: f32) -> usize {
        (((height - self.bottom) / RING_HEIGHT).floor().max(0.0) as usize).min(self.ring_count - 1)
    }

    fn start_of(&self, ring: usize) -> f32 {
        self.bottom + RING_HEIGHT * ring as f32
    }

    fn cell_of(&self, point: Vector3<f32>) -> (usize, usize, f32) {
        let offset = point - self.axis_center;
        let along_reference = offset.dot(self.reference);
        let along_side = offset.dot(self.side);
        let angle = along_side.atan2(along_reference) + std::f32::consts::PI;
        let sector =
            ((angle / std::f32::consts::TAU) * SECTOR_COUNT as f32) as usize % SECTOR_COUNT;
        let radius = (along_reference * along_reference + along_side * along_side).sqrt();
        (self.ring_of(point.dot(world_y_axis())), sector, radius)
    }
}

/// The largest radius seen in each (ring, sector) cell.
struct RadiusTable {
    cells: Vec<Option<f32>>,
}

impl RadiusTable {
    fn new(ring_count: usize) -> Self {
        Self {
            cells: vec![None; ring_count * SECTOR_COUNT],
        }
    }

    fn record(&mut self, ring: usize, sector: usize, radius: f32) {
        let cell = &mut self.cells[ring * SECTOR_COUNT + sector];
        *cell = Some(cell.map_or(radius, |seen| seen.max(radius)));
    }

    fn get(&self, ring: usize, sector: usize) -> Option<f32> {
        self.cells[ring * SECTOR_COUNT + sector]
    }

    fn ring_count(&self) -> usize {
        self.cells.len() / SECTOR_COUNT
    }

    /// Per cell, the largest radius of the same sector within the rings a ring would drop onto.
    fn shelf_below(&self) -> RadiusTable {
        let mut below = RadiusTable::new(self.ring_count());
        for sector in 0..SECTOR_COUNT {
            for ring in 0..self.ring_count() {
                let shelf = (ring.saturating_sub(SHELF_REACH_RINGS)..ring)
                    .filter_map(|lower| self.get(lower, sector))
                    .reduce(f32::max);
                below.cells[ring * SECTOR_COUNT + sector] = shelf;
            }
        }
        below
    }
}

struct BodyPoint {
    position: Vector3<f32>,
    bone: BoneId,
}

fn heaviest_bone(influences: &[(BoneId, f32)]) -> Option<BoneId> {
    influences
        .iter()
        .filter(|(_, weight)| *weight > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(bone, _)| *bone)
}

fn body_points(surfaces: &[SkinSurface]) -> Vec<BodyPoint> {
    surfaces
        .iter()
        .flat_map(|surface| surface.positions.iter().zip(&surface.influences))
        .filter_map(|(position, influences)| {
            Some(BodyPoint {
                position: *position,
                bone: heaviest_bone(influences)?,
            })
        })
        .collect()
}

fn height_range(points: impl Iterator<Item = f32>) -> Option<(f32, f32)> {
    points.fold(None, |range, height| match range {
        None => Some((height, height)),
        Some((low, high)) => Some((low.min(height), high.max(height))),
    })
}

/// Whether the garment ring rests on the body: fitted against it, or narrower than the shelf just below it.
fn sector_supported(
    garment_radius: f32,
    body_radius: Option<f32>,
    body_radius_below: Option<f32>,
    fit_gap: f32,
) -> bool {
    let fitted = body_radius.is_some_and(|body| garment_radius - body <= fit_gap);
    let caught = body_radius_below.is_some_and(|below| garment_radius <= below + fit_gap);
    fitted || caught
}

fn ring_supported(
    ring: usize,
    garment: &RadiusTable,
    body: &RadiusTable,
    body_below: &RadiusTable,
    params: &ChainGenParams,
) -> bool {
    let settings = &params.settings.garment;
    let mut defined = 0usize;
    let mut supported = 0usize;
    for sector in 0..SECTOR_COUNT {
        let Some(garment_radius) = garment.get(ring, sector) else {
            continue;
        };
        defined += 1;
        if sector_supported(
            garment_radius,
            body.get(ring, sector),
            body_below.get(ring, sector),
            settings.fit_gap,
        ) {
            supported += 1;
        }
    }
    defined > 0 && supported as f32 >= settings.support_ratio * defined as f32
}

/// The bone most of the body points in the highest occupied ring at or below `ring` belong to.
fn supporting_bone(points: &[BodyPoint], grid: &RingGrid, ring: usize) -> Option<BoneId> {
    let highest_ring = points
        .iter()
        .map(|point| grid.cell_of(point.position).0)
        .filter(|&point_ring| point_ring <= ring)
        .max()?;
    let mut votes: BTreeMap<BoneId, usize> = BTreeMap::new();
    for point in points {
        if grid.cell_of(point.position).0 == highest_ring {
            *votes.entry(point.bone).or_default() += 1;
        }
    }
    votes
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(bone, _)| bone)
}

/// The lowest ring of the garment the body supports, or None when the body supports none of it.
/// Fails when the body supports every ring, because no chain is left.
pub(super) fn garment_support(
    input: &ChainGenInput,
    params: &ChainGenParams,
    vertices: &[u32],
) -> anyhow::Result<Option<GarmentSupport>> {
    let body = body_points(input.skins);
    if body.is_empty() {
        return Ok(None);
    }
    let up = world_y_axis();
    let garment_positions: Vec<Vector3<f32>> = vertices
        .iter()
        .map(|&vertex| input.positions[vertex as usize])
        .collect();
    let Some((garment_bottom, garment_top)) =
        height_range(garment_positions.iter().map(|p| p.dot(up)))
    else {
        return Ok(None);
    };
    let body_bottom = height_range(body.iter().map(|p| p.position.dot(up)))
        .map_or(garment_bottom, |(low, _)| low.min(garment_bottom));
    let axis_center = centroid(&garment_positions).unwrap_or(input.root_position);
    let grid = RingGrid::new(body_bottom, garment_top, axis_center);

    let mut garment_radii = RadiusTable::new(grid.ring_count);
    for position in &garment_positions {
        let (ring, sector, radius) = grid.cell_of(*position);
        garment_radii.record(ring, sector, radius);
    }
    let mut body_radii = RadiusTable::new(grid.ring_count);
    for point in &body {
        let (ring, sector, radius) = grid.cell_of(point.position);
        body_radii.record(ring, sector, radius);
    }
    let body_below = body_radii.shelf_below();

    let lowest_garment_ring = grid.ring_of(garment_bottom);
    let supported_ring = (lowest_garment_ring..grid.ring_count)
        .find(|&ring| ring_supported(ring, &garment_radii, &body_radii, &body_below, params));
    let Some(ring) = supported_ring else {
        return Ok(None);
    };
    if ring == lowest_garment_ring {
        anyhow::bail!("the body supports every ring of the garment; no chain is needed");
    }
    let bone = supporting_bone(&body, &grid, ring).unwrap_or(input.root_bone);
    Ok(Some(GarmentSupport {
        boundary_height: grid.start_of(ring),
        bone,
    }))
}

/// The influences of the body point nearest to `point`.
pub(super) fn nearest_body_influences(
    surfaces: &[SkinSurface],
    point: Vector3<f32>,
) -> Option<Vec<(BoneId, f32)>> {
    surfaces
        .iter()
        .flat_map(|surface| surface.positions.iter().zip(&surface.influences))
        .filter(|(_, influences)| !influences.is_empty())
        .min_by(|a, b| {
            (a.0 - point)
                .magnitude2()
                .total_cmp(&(b.0 - point).magnitude2())
        })
        .map(|(_, influences)| influences.clone())
}
