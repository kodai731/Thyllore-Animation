use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pythonize::{depythonize, PythonizeError};

use super::chain_gen::pythonize_value;
use super::error::anyhow_to_pyerr;
use crate::avatar_setup_bridge::{
    infer_humanoid_mapping as bridge_infer_humanoid_mapping, sidecar_build as bridge_sidecar_build,
    sidecar_read as bridge_sidecar_read, spring_joint_params, BoneSpec, HumanoidMappingOut,
    SidecarBuildInput,
};
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;

#[pyfunction]
pub fn spring_joint_ui_params(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    pythonize_value(py, &spring_joint_params())
}

#[pyfunction]
pub fn sidecar_read<'py>(py: Python<'py>, json: &str) -> PyResult<Bound<'py, PyAny>> {
    let sidecar = bridge_sidecar_read(json).map_err(anyhow_to_pyerr)?;
    pythonize_value(py, &sidecar)
}

#[pyfunction]
pub fn sidecar_build(input: &Bound<PyAny>) -> PyResult<String> {
    let input: SidecarBuildInput = depythonize(input).map_err(depythonize_error_to_pyerr)?;
    bridge_sidecar_build(&input).map_err(|e| PyValueError::new_err(format!("{:#}", e)))
}

#[pyfunction]
pub fn infer_humanoid_mapping<'py>(
    py: Python<'py>,
    bones: &Bound<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let bone_inputs = bones_from_py(bones)?;
    let output: HumanoidMappingOut = bridge_infer_humanoid_mapping(&bone_inputs);
    pythonize_value(py, &output)
}

pub(super) fn bones_from_py(bones: &Bound<PyAny>) -> PyResult<Vec<BoneInput>> {
    let specs: Vec<BoneSpec> = depythonize(bones).map_err(depythonize_error_to_pyerr)?;
    Ok(specs.into_iter().map(Into::into).collect())
}

pub(super) fn depythonize_error_to_pyerr(error: PythonizeError) -> PyErr {
    PyValueError::new_err(error.to_string())
}
