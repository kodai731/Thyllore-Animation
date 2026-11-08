use pyo3::prelude::*;
use pythonize::depythonize;

use crate::blender_space::{
    armature_local_points as to_armature_local, bone_specs as bone_specs_of, BoneSpecsRequest,
    MatrixRows,
};
use thyllore_ml_api::python::{anyhow_to_pyerr, depythonize_error_to_pyerr, pythonize_value};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ArmatureLocalPointsRequest {
    armature_world: MatrixRows,
    points: Vec<[f32; 3]>,
}

/// The bone specs of an armature (`name`, `parent`, `rest_position`, `rest_frame`, `bind_matrix`) from
/// its world matrix and the bones' `matrix_local` rows; the one rig read every other call takes.
#[pyfunction]
pub fn bone_specs<'py>(py: Python<'py>, request: &Bound<PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let request: BoneSpecsRequest = depythonize(request).map_err(depythonize_error_to_pyerr)?;
    let specs = bone_specs_of(&request).map_err(anyhow_to_pyerr)?;
    pythonize_value(py, &specs)
}

/// Blender world points in the armature's own space, where `EditBone.head` and `tail` are set.
#[pyfunction]
pub fn armature_local_points(request: &Bound<PyAny>) -> PyResult<Vec<[f32; 3]>> {
    let request: ArmatureLocalPointsRequest =
        depythonize(request).map_err(depythonize_error_to_pyerr)?;
    to_armature_local(&request.armature_world, &request.points).map_err(anyhow_to_pyerr)
}
