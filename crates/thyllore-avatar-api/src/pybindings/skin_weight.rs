use glam::Vec3;
use pyo3::prelude::*;
use pythonize::depythonize;
use thyllore_math_core::{
    transfer_skin_weights as math_transfer_skin_weights, TargetSurface, WeightTransferParams,
    WeightedSurface,
};

use thyllore_ml_api::python::{anyhow_to_pyerr, depythonize_error_to_pyerr};

#[derive(serde::Deserialize)]
struct SurfaceIn {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    #[serde(default)]
    weights: Vec<Vec<f32>>,
}

fn to_vectors(points: &[[f32; 3]]) -> Vec<Vec3> {
    points.iter().copied().map(Vec3::from).collect()
}

/// The weights of every target vertex transferred from the skinned source surface; both surfaces
/// are in the same space and `weights` is one row per source vertex.
#[pyfunction]
#[pyo3(signature = (source, target, max_distance, max_angle_deg, max_iterations = 100, tolerance = 1e-6))]
pub fn transfer_skin_weights(
    source: &Bound<PyAny>,
    target: &Bound<PyAny>,
    max_distance: f32,
    max_angle_deg: f32,
    max_iterations: usize,
    tolerance: f64,
) -> PyResult<Vec<Vec<f32>>> {
    let source: SurfaceIn = depythonize(source).map_err(depythonize_error_to_pyerr)?;
    let target: SurfaceIn = depythonize(target).map_err(depythonize_error_to_pyerr)?;
    let (source_positions, source_normals) =
        (to_vectors(&source.positions), to_vectors(&source.normals));
    let (target_positions, target_normals) =
        (to_vectors(&target.positions), to_vectors(&target.normals));

    math_transfer_skin_weights(
        &WeightedSurface {
            positions: &source_positions,
            normals: &source_normals,
            triangles: &source.triangles,
            weights: &source.weights,
        },
        &TargetSurface {
            positions: &target_positions,
            normals: &target_normals,
            triangles: &target.triangles,
        },
        &WeightTransferParams {
            max_distance,
            max_angle_deg,
            max_iterations,
            tolerance,
        },
    )
    .map_err(anyhow_to_pyerr)
}
