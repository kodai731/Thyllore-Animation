use crate::volume::RayKnots;
use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::mix;

// One cell grid spans the whole ray so a knot splitting a piece never moves a sample; the cell
// length comes from the active length (active pieces) so the budget is spent on density.
pub const MODULATION_CELLS: usize = 64;
pub const ACTIVE_CELLS_MIN: usize = 16;
const MODULATION_SAMPLE_FRACTION: f32 = 0.125;
pub(crate) const EMPTY_INTERVAL_EPSILON: f32 = 1e-6;

/// Cell length along the ray: a fraction of the finest modulation feature (world units, the active
/// span when none), bounded so the active length holds between ACTIVE_CELLS_MIN and MODULATION_CELLS cells.
pub fn modulation_step(
    direction: Vector3<f32>,
    active_length: f32,
    finest_feature: Option<f32>,
) -> f32 {
    let span = active_length.max(EMPTY_INTERVAL_EPSILON);
    let active_span = span * direction.magnitude();
    let feature = finest_feature.map_or(active_span, |feature| feature.min(active_span));
    (MODULATION_SAMPLE_FRACTION * feature / direction.magnitude()).clamp(
        span / MODULATION_CELLS as f32,
        span / ACTIVE_CELLS_MIN as f32,
    )
}

/// A medium integrated piece by piece in closed form under a cell-sampled linear modulation.
pub trait RayMedium {
    type Ray;

    fn ray_pieces(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        t_near: f32,
        t_far: f32,
    ) -> (RayKnots, Self::Ray);

    fn piece_is_active(
        &self,
        ray: &Self::Ray,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        s0: f32,
        s1: f32,
    ) -> bool;

    fn modulation_at(&self, point: Vector3<f32>, step_ahead: Vector3<f32>) -> f32;

    fn modulation_step(&self, direction: Vector3<f32>, active_length: f32) -> f32;

    fn piece_optical_depth(
        &self,
        ray: &Self::Ray,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        s0: f32,
        s1: f32,
        modulation: (f32, f32),
    ) -> f32;
}

/// Length of the ray inside active pieces: continuous in the ray, so the cell length derived
/// from it never jumps when a knot appears.
fn measure_active_length<M: RayMedium>(
    medium: &M,
    ray: &M::Ray,
    knots: &RayKnots,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
) -> f32 {
    knots
        .pieces()
        .filter(|&(s0, s1)| medium.piece_is_active(ray, origin, direction, s0, s1))
        .map(|(s0, s1)| s1 - s0)
        .sum()
}

/// Modulation at both nodes of a cell; a node shared with the previous cell is not re-evaluated.
#[derive(Clone, Copy)]
struct CellModulation {
    cell: Option<i64>,
    a: f32,
    b: f32,
}

impl CellModulation {
    fn none() -> Self {
        Self {
            cell: None,
            a: 1.0,
            b: 1.0,
        }
    }

    fn advance<M: RayMedium>(
        self,
        medium: &M,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        t_near: f32,
        step: f32,
        cell: i64,
    ) -> Self {
        if self.cell == Some(cell) {
            return self;
        }
        let cell_start = t_near + cell as f32 * step;
        let a = if self.cell == Some(cell - 1) {
            self.b
        } else {
            medium.modulation_at(origin + direction * cell_start, direction * step)
        };
        let b = medium.modulation_at(origin + direction * (cell_start + step), direction * step);
        Self {
            cell: Some(cell),
            a,
            b,
        }
    }
}

/// Pieces are walked in ray order and each is cut by the cells of the ray-wide grid it overlaps.
pub fn cell_modulated_optical_depth<M: RayMedium>(
    medium: &M,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> f32 {
    if t_far <= t_near {
        return 0.0;
    }
    let (knots, ray) = medium.ray_pieces(origin, direction, t_near, t_far);
    let active_length = measure_active_length(medium, &ray, &knots, origin, direction);
    if active_length <= EMPTY_INTERVAL_EPSILON {
        return 0.0;
    }
    let step = medium.modulation_step(direction, active_length);

    let mut total = 0.0f32;
    let mut modulation = CellModulation::none();
    for (piece_start, piece_end) in knots.pieces() {
        if !medium.piece_is_active(&ray, origin, direction, piece_start, piece_end) {
            continue;
        }
        let cell_first = ((piece_start - t_near) / step).floor() as i64;
        let cell_last = (((piece_end - t_near) / step).floor() as i64)
            .min(cell_first + MODULATION_CELLS as i64);
        for cell in cell_first..=cell_last {
            let cell_start = t_near + cell as f32 * step;
            let s0 = piece_start.max(cell_start);
            let s1 = piece_end.min(cell_start + step);
            if s1 <= s0 {
                continue;
            }
            modulation = modulation.advance(medium, origin, direction, t_near, step, cell);
            let modulation_0 = mix(modulation.a, modulation.b, (s0 - cell_start) / step);
            let modulation_1 = mix(modulation.a, modulation.b, (s1 - cell_start) / step);
            total += medium.piece_optical_depth(
                &ray,
                origin,
                direction,
                s0,
                s1,
                (modulation_0, modulation_1),
            );
        }
    }
    total
}
