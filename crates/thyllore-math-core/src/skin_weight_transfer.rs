//! Robust Skin Weights Transfer (Abdrashitov et al., SIGGRAPH Asia 2023).
use glam::{DVec3, Vec3};

use crate::connected_components::triangle_component_labels;
use crate::triangle::barycentric_coordinates;
use crate::triangle_bvh::TriangleBvh;

const DEGENERATE_AREA_EPSILON: f64 = 1e-12;

/// A skinned surface: one weight row per vertex, `bone_count` columns.
pub struct WeightedSurface<'a> {
    pub positions: &'a [Vec3],
    pub normals: &'a [Vec3],
    pub triangles: &'a [[u32; 3]],
    pub weights: &'a [Vec<f32>],
}

/// The surface that receives the weights.
pub struct TargetSurface<'a> {
    pub positions: &'a [Vec3],
    pub normals: &'a [Vec3],
    pub triangles: &'a [[u32; 3]],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeightTransferParams {
    /// A target vertex farther than this from the source surface gets inpainted weights.
    pub max_distance: f32,
    /// A target vertex whose normal deviates more than this from the source gets inpainted weights.
    pub max_angle_deg: f32,
    pub max_iterations: usize,
    pub tolerance: f64,
}

/// The nearest source surface point of a target vertex: the weights and the normal there.
struct SurfaceMatch {
    distance: f32,
    weights: Vec<f64>,
    normal: Vec3,
}

/// Weights for every target vertex: copied from the nearest source surface where the surfaces
/// coincide, and solved by the cotangent Laplacian from those vertices elsewhere.
pub fn transfer_skin_weights(
    source: &WeightedSurface,
    target: &TargetSurface,
    params: &WeightTransferParams,
) -> anyhow::Result<Vec<Vec<f32>>> {
    let bone_count = source.weights.first().map_or(0, Vec::len);
    anyhow::ensure!(
        source.weights.iter().all(|row| row.len() == bone_count),
        "every source weight row must have {bone_count} bones"
    );
    anyhow::ensure!(
        source.positions.len() == source.normals.len()
            && source.positions.len() == source.weights.len(),
        "source positions, normals and weights must have one entry per vertex"
    );
    anyhow::ensure!(
        target.positions.len() == target.normals.len(),
        "target positions and normals must have one entry per vertex"
    );

    let bvh = TriangleBvh::build(source.positions, source.triangles);
    let matches: Vec<SurfaceMatch> = target
        .positions
        .iter()
        .map(|&point| nearest_surface_match(source, &bvh, point, bone_count))
        .collect();

    let matched: Vec<bool> = matches
        .iter()
        .zip(target.normals)
        .map(|(surface, &normal)| is_matched(surface.distance, normal, surface.normal, params))
        .collect();
    let interpolated: Vec<Vec<f64>> = matches.into_iter().map(|m| m.weights).collect();

    let inpainted = inpaint_weights(
        target.positions,
        target.triangles,
        &interpolated,
        &matched,
        params.max_iterations,
        params.tolerance,
    );
    Ok(inpainted
        .into_iter()
        .map(|row| row.into_iter().map(|w| w as f32).collect())
        .collect())
}

fn nearest_surface_match(
    source: &WeightedSurface,
    bvh: &TriangleBvh,
    point: Vec3,
    bone_count: usize,
) -> SurfaceMatch {
    let Some(surface) = bvh.closest_point(point, f32::INFINITY) else {
        return SurfaceMatch {
            distance: f32::INFINITY,
            weights: vec![0.0; bone_count],
            normal: Vec3::new(0.0, 0.0, 0.0),
        };
    };
    let corners = source.triangles[surface.triangle as usize];
    let [a, b, c] = corners.map(|v| source.positions[v as usize]);
    let barycentric = barycentric_coordinates(surface.point, a, b, c);

    let mut weights = vec![0.0f64; bone_count];
    let mut normal = Vec3::new(0.0, 0.0, 0.0);
    for (corner, &coordinate) in corners.iter().zip(barycentric.as_ref() as &[f32; 3]) {
        for (sum, &w) in weights.iter_mut().zip(&source.weights[*corner as usize]) {
            *sum += coordinate as f64 * w as f64;
        }
        normal += source.normals[*corner as usize] * coordinate;
    }
    SurfaceMatch {
        distance: surface.distance,
        weights,
        normal,
    }
}

/// Whether a target vertex lies on the source surface: near it, with a normal within
/// `max_angle_deg` of the surface normal in either direction.
pub fn is_matched(
    distance: f32,
    target_normal: Vec3,
    source_normal: Vec3,
    params: &WeightTransferParams,
) -> bool {
    if distance > params.max_distance {
        return false;
    }
    let cos_angle = target_normal
        .normalize()
        .dot(source_normal.normalize())
        .clamp(-1.0, 1.0);
    let angle_deg = cos_angle.acos().to_degrees();
    angle_deg <= params.max_angle_deg || angle_deg >= 180.0 - params.max_angle_deg
}

fn double_area(a: DVec3, b: DVec3, c: DVec3) -> f64 {
    (b - a).cross(c - a).length()
}

fn to_f64(v: Vec3) -> DVec3 {
    v.as_dvec3()
}

fn nondegenerate_triangles(positions: &[Vec3], triangles: &[[u32; 3]]) -> Vec<[u32; 3]> {
    triangles
        .iter()
        .copied()
        .filter(|triangle| {
            let [a, b, c] = triangle.map(|v| to_f64(positions[v as usize]));
            double_area(a, b, c) > DEGENERATE_AREA_EPSILON
        })
        .collect()
}

/// A sparse matrix as `(row, column, value)` triplets; repeated entries add up.
pub struct SparseTriplets {
    pub rows: Vec<usize>,
    pub cols: Vec<usize>,
    pub values: Vec<f64>,
}

impl SparseTriplets {
    fn push(&mut self, row: usize, col: usize, value: f64) {
        self.rows.push(row);
        self.cols.push(col);
        self.values.push(value);
    }

    fn multiply(&self, x: &[f64], size: usize) -> Vec<f64> {
        let mut y = vec![0.0; size];
        for ((&row, &col), &value) in self.rows.iter().zip(&self.cols).zip(&self.values) {
            y[row] += value * x[col];
        }
        y
    }
}

/// The cotangent Laplacian of the mesh: symmetric, with zero row sums.
pub fn cotangent_laplacian(positions: &[Vec3], triangles: &[[u32; 3]]) -> SparseTriplets {
    let mut laplacian = SparseTriplets {
        rows: Vec::new(),
        cols: Vec::new(),
        values: Vec::new(),
    };
    for triangle in nondegenerate_triangles(positions, triangles) {
        let corners = triangle.map(|v| to_f64(positions[v as usize]));
        let area2 = double_area(corners[0], corners[1], corners[2]);
        for (opposite, i, j) in [(0, 1, 2), (1, 2, 0), (2, 0, 1)] {
            let to_i = corners[i] - corners[opposite];
            let to_j = corners[j] - corners[opposite];
            let half_cot = 0.5 * to_i.dot(to_j) / area2;
            let (vi, vj) = (triangle[i] as usize, triangle[j] as usize);
            laplacian.push(vi, vj, -half_cot);
            laplacian.push(vj, vi, -half_cot);
            laplacian.push(vi, vi, half_cot);
            laplacian.push(vj, vj, half_cot);
        }
    }
    laplacian
}

fn solve_conjugate_gradient(
    system: &SparseTriplets,
    rhs: &[f64],
    max_iterations: usize,
    tolerance: f64,
) -> Vec<f64> {
    let size = rhs.len();
    let mut x = vec![0.0; size];
    let mut residual = rhs.to_vec();
    let mut direction = residual.clone();
    let mut residual_norm_sq: f64 = residual.iter().map(|r| r * r).sum();

    for _ in 0..max_iterations {
        if residual_norm_sq.sqrt() <= tolerance {
            break;
        }
        let projected = system.multiply(&direction, size);
        let curvature: f64 = direction.iter().zip(&projected).map(|(d, p)| d * p).sum();
        if curvature == 0.0 {
            break;
        }
        let alpha = residual_norm_sq / curvature;
        for ((xi, di), (ri, pi)) in x
            .iter_mut()
            .zip(&direction)
            .zip(residual.iter_mut().zip(&projected))
        {
            *xi += alpha * di;
            *ri -= alpha * pi;
        }
        let next_norm_sq: f64 = residual.iter().map(|r| r * r).sum();
        let beta = next_norm_sq / residual_norm_sq;
        for (di, ri) in direction.iter_mut().zip(&residual) {
            *di = ri + beta * *di;
        }
        residual_norm_sq = next_norm_sq;
    }
    x
}

fn normalize_rows(weights: &mut [Vec<f64>]) {
    for row in weights {
        for w in row.iter_mut() {
            *w = w.clamp(0.0, 1.0);
        }
        let sum: f64 = row.iter().sum();
        if sum > 0.0 {
            for w in row.iter_mut() {
                *w /= sum;
            }
        }
    }
}

/// The vertices whose component has no matched vertex: with no boundary to inpaint from, they
/// keep the weights interpolated from the nearest source surface.
fn components_without_match(
    positions: &[Vec3],
    triangles: &[[u32; 3]],
    matched: &[bool],
) -> Vec<bool> {
    let labels = triangle_component_labels(
        positions.len(),
        &nondegenerate_triangles(positions, triangles),
    );
    let mut component_has_match = vec![false; positions.len()];
    for (vertex, &is_match) in matched.iter().enumerate() {
        if is_match {
            component_has_match[labels[vertex]] = true;
        }
    }
    labels
        .iter()
        .map(|&label| !component_has_match[label])
        .collect()
}

/// Fills the unmatched vertices by harmonic interpolation of the matched ones.
pub fn inpaint_weights(
    positions: &[Vec3],
    triangles: &[[u32; 3]],
    weights: &[Vec<f64>],
    matched: &[bool],
    max_iterations: usize,
    tolerance: f64,
) -> Vec<Vec<f64>> {
    let isolated = components_without_match(positions, triangles, matched);
    let fixed: Vec<bool> = matched
        .iter()
        .zip(&isolated)
        .map(|(&m, &i)| m || i)
        .collect();
    if fixed.iter().all(|&f| f) {
        return weights.to_vec();
    }

    let laplacian = cotangent_laplacian(positions, triangles);
    let unknown: Vec<usize> = (0..positions.len()).filter(|&v| !fixed[v]).collect();
    let mut unknown_index = vec![None; positions.len()];
    for (slot, &vertex) in unknown.iter().enumerate() {
        unknown_index[vertex] = Some(slot);
    }

    let mut unknown_system = SparseTriplets {
        rows: Vec::new(),
        cols: Vec::new(),
        values: Vec::new(),
    };
    let mut boundary_terms: Vec<(usize, usize, f64)> = Vec::new();
    for ((&row, &col), &value) in laplacian
        .rows
        .iter()
        .zip(&laplacian.cols)
        .zip(&laplacian.values)
    {
        let Some(row_slot) = unknown_index[row] else {
            continue;
        };
        match unknown_index[col] {
            Some(col_slot) => unknown_system.push(row_slot, col_slot, value),
            None => boundary_terms.push((row_slot, col, value)),
        }
    }

    let bone_count = weights.first().map_or(0, Vec::len);
    let mut result = weights.to_vec();
    for bone in 0..bone_count {
        let mut rhs = vec![0.0; unknown.len()];
        for &(row_slot, fixed_vertex, value) in &boundary_terms {
            rhs[row_slot] -= value * weights[fixed_vertex][bone];
        }
        let solved = solve_conjugate_gradient(&unknown_system, &rhs, max_iterations, tolerance);
        for (&vertex, value) in unknown.iter().zip(solved) {
            result[vertex][bone] = value;
        }
    }
    normalize_rows(&mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    const PARAMS: WeightTransferParams = WeightTransferParams {
        max_distance: 0.1,
        max_angle_deg: 30.0,
        max_iterations: 100,
        tolerance: 1e-6,
    };

    #[test]
    fn the_laplacian_is_symmetric_with_zero_row_sums() {
        let positions = [
            v(0.0, 0.0, 0.0),
            v(1.0, 0.0, 0.0),
            v(1.0, 1.0, 0.0),
            v(0.0, 1.0, 0.0),
        ];
        let laplacian = cotangent_laplacian(&positions, &[[0, 1, 2], [0, 2, 3]]);

        let mut dense = [[0.0f64; 4]; 4];
        for ((&r, &c), &value) in laplacian
            .rows
            .iter()
            .zip(&laplacian.cols)
            .zip(&laplacian.values)
        {
            dense[r][c] += value;
        }
        for r in 0..4 {
            assert!(dense[r].iter().sum::<f64>().abs() < 1e-10);
            for c in 0..4 {
                assert!((dense[r][c] - dense[c][r]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn inpainting_decreases_monotonically_between_two_fixed_ends() {
        let positions = [
            v(0.0, 0.0, 0.0),
            v(1.0, 0.0, 0.0),
            v(2.0, 0.0, 0.0),
            v(3.0, 0.0, 0.0),
            v(4.0, 0.0, 0.0),
            v(0.0, -1.0, 0.0),
        ];
        let triangles = [[0, 1, 5], [1, 2, 5], [2, 3, 5], [3, 4, 5]];
        let mut weights = vec![vec![0.0, 0.0]; 6];
        weights[0][0] = 1.0;
        weights[4][1] = 1.0;
        let matched = [true, false, false, false, true, false];

        let result = inpaint_weights(&positions, &triangles, &weights, &matched, 100, 1e-6);

        for row in &result {
            assert!((row.iter().sum::<f64>() - 1.0).abs() < 1e-3);
        }
        for pair in result[..5].windows(2) {
            assert!(pair[0][0] >= pair[1][0] - 1e-6);
        }
    }

    #[test]
    fn a_match_needs_both_distance_and_normal_agreement() {
        let x = v(1.0, 0.0, 0.0);
        assert!(is_matched(0.01, x, x, &PARAMS));
        assert!(!is_matched(0.5, v(0.0, 1.0, 0.0), x, &PARAMS));
        assert!(!is_matched(0.01, v(0.0, 1.0, 0.0), x, &PARAMS));
        assert!(is_matched(0.01, v(-1.0, 0.0, 0.0), x, &PARAMS));
    }

    #[test]
    fn a_component_without_a_match_keeps_its_interpolated_weights() {
        let positions = [
            v(0.0, 0.0, 0.0),
            v(1.0, 0.0, 0.0),
            v(0.0, 1.0, 0.0),
            v(5.0, 0.0, 0.0),
            v(6.0, 0.0, 0.0),
            v(5.0, 1.0, 0.0),
        ];
        let triangles = [[0, 1, 2], [3, 4, 5]];
        let weights = vec![
            vec![1.0, 0.0],
            vec![0.5, 0.5],
            vec![0.5, 0.5],
            vec![0.0, 1.0],
            vec![0.2, 0.8],
            vec![0.4, 0.6],
        ];
        let matched = [true, false, false, false, false, false];

        let result = inpaint_weights(&positions, &triangles, &weights, &matched, 100, 1e-6);

        for (row, expected) in result[3..].iter().zip(&weights[3..]) {
            for (a, b) in row.iter().zip(expected) {
                assert!((a - b).abs() < 1e-9);
            }
        }
        for row in &result {
            assert!((row.iter().sum::<f64>() - 1.0).abs() < 1e-3);
        }
    }

    #[test]
    fn a_coincident_target_copies_the_source_weights() {
        let positions = [v(0.0, 0.0, 0.0), v(2.0, 0.0, 0.0), v(0.0, 2.0, 0.0)];
        let normals = [v(0.0, 0.0, 1.0); 3];
        let triangles = [[0, 1, 2]];
        let weights = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![0.5, 0.5]];
        let source = WeightedSurface {
            positions: &positions,
            normals: &normals,
            triangles: &triangles,
            weights: &weights,
        };
        let target_positions = [v(1.0, 0.0, 0.0)];
        let target = TargetSurface {
            positions: &target_positions,
            normals: &normals[..1],
            triangles: &[],
        };

        let result = transfer_skin_weights(&source, &target, &PARAMS).unwrap();

        assert!((result[0][0] - 0.5).abs() < 1e-6);
        assert!((result[0][1] - 0.5).abs() < 1e-6);
    }
}
