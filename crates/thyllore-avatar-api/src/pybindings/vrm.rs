use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pythonize::depythonize;

use std::collections::BTreeMap;

use super::avatar_setup::bones_from_py;
use crate::vrm_bridge::{
    vrm_t_pose_warnings as bridge_t_pose_warnings, vrm_write as bridge_vrm_write, VrmWriteRequest,
};
use thyllore_avatar_core::vrm::{
    vrm_meta_fields as avatar_vrm_meta_fields, VrmMetaField, VRM_META_DEFAULT_VERSION,
};
use thyllore_ml_api::python::{anyhow_to_pyerr, depythonize_error_to_pyerr, pythonize_value};

#[derive(serde::Serialize)]
struct VrmMetaFieldsOut {
    default_version: &'static str,
    fields: Vec<VrmMetaField>,
}

/// The editable `VRMC_vrm.meta` fields with their keys, labels, kinds and choices.
#[pyfunction]
pub fn vrm_meta_fields(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    pythonize_value(
        py,
        &VrmMetaFieldsOut {
            default_version: VRM_META_DEFAULT_VERSION,
            fields: avatar_vrm_meta_fields(),
        },
    )
}

#[pyfunction]
pub fn vrm_t_pose_warnings(
    bones: &Bound<PyAny>,
    humanoid_overrides: &Bound<PyAny>,
) -> PyResult<Vec<String>> {
    let bone_inputs = bones_from_py(bones)?;
    let overrides: BTreeMap<String, String> =
        depythonize(humanoid_overrides).map_err(depythonize_error_to_pyerr)?;
    bridge_t_pose_warnings(&bone_inputs, &overrides).map_err(anyhow_to_pyerr)
}

#[pyfunction]
pub fn vrm_write<'py>(py: Python<'py>, request: &Bound<PyAny>) -> PyResult<Bound<'py, PyBytes>> {
    let request: VrmWriteRequest = depythonize(request).map_err(depythonize_error_to_pyerr)?;
    let vrm = bridge_vrm_write(&request).map_err(anyhow_to_pyerr)?;
    Ok(PyBytes::new(py, &vrm))
}
