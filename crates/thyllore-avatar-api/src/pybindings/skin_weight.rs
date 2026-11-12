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

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TransferSkinWeightsRequest {
    source: SurfaceIn,
    target: SurfaceIn,
    max_distance: f32,
    max_angle_deg: f32,
    #[serde(default = "default_max_iterations")]
    max_iterations: usize,
    #[serde(default = "default_tolerance")]
    tolerance: f64,
}

fn default_max_iterations() -> usize {
    100
}

fn default_tolerance() -> f64 {
    1e-6
}

fn to_vectors(points: &[[f32; 3]]) -> Vec<Vec3> {
    points.iter().copied().map(Vec3::from).collect()
}

/// The weights of every target vertex transferred from the skinned source surface; both surfaces
/// are in the same space and `weights` is one row per source vertex.
#[pyfunction]
pub fn transfer_skin_weights(request: &Bound<PyAny>) -> PyResult<Vec<Vec<f32>>> {
    let request: TransferSkinWeightsRequest =
        depythonize(request).map_err(depythonize_error_to_pyerr)?;
    let (source, target) = (&request.source, &request.target);
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
            max_distance: request.max_distance,
            max_angle_deg: request.max_angle_deg,
            max_iterations: request.max_iterations,
            tolerance: request.tolerance,
        },
    )
    .map_err(anyhow_to_pyerr)
}
