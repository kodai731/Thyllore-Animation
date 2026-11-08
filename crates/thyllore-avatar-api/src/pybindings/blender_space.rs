use pyo3::prelude::*;
use pythonize::depythonize;

use crate::blender_space::{armature_local_points as to_armature_local, MatrixRows};
use thyllore_ml_api::python::{anyhow_to_pyerr, depythonize_error_to_pyerr};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ArmatureLocalPointsRequest {
    armature_world: MatrixRows,
    points: Vec<[f32; 3]>,
}

/// Blender world points in the armature's own space, where `EditBone.head` and `tail` are set.
#[pyfunction]
pub fn armature_local_points(request: &Bound<PyAny>) -> PyResult<Vec<[f32; 3]>> {
    let request: ArmatureLocalPointsRequest =
        depythonize(request).map_err(depythonize_error_to_pyerr)?;
    to_armature_local(&request.armature_world, &request.points).map_err(anyhow_to_pyerr)
}
