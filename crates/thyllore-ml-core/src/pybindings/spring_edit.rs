use pyo3::prelude::*;
use pythonize::depythonize;

use super::avatar_setup::{bones_from_py, depythonize_error_to_pyerr};
use super::chain_gen::pythonize_value;
use crate::spring_edit_bridge::{
    apply_ramp_to_chains, chains_from_prefix as bridge_chains_from_prefix,
    chains_in_scope as bridge_chains_in_scope, chains_under_bone as bridge_chains_under_bone,
    RampInput,
};

#[pyfunction]
pub fn chains_from_prefix<'py>(
    py: Python<'py>,
    spring: &Bound<PyAny>,
    bones: &Bound<PyAny>,
    prefix: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let spring: thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring =
        depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bone_inputs = bones_from_py(bones)?;
    let output = bridge_chains_from_prefix(&spring, &bone_inputs, prefix)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))?;
    pythonize_value(py, &output)
}

#[pyfunction]
pub fn apply_ramp<'py>(
    py: Python<'py>,
    spring: &Bound<PyAny>,
    bones: &Bound<PyAny>,
    chain_names: Vec<String>,
    ramp: &Bound<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let spring: thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring =
        depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bone_inputs = bones_from_py(bones)?;
    let ramp_input: RampInput = depythonize(ramp).map_err(depythonize_error_to_pyerr)?;
    let output = apply_ramp_to_chains(&spring, &bone_inputs, &chain_names, &ramp_input)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))?;
    pythonize_value(py, &output)
}

#[pyfunction]
pub fn chains_in_scope(
    _py: Python<'_>,
    spring: &Bound<PyAny>,
    bones: &Bound<PyAny>,
    chain_name: &str,
    scope: &str,
) -> PyResult<Vec<String>> {
    let spring: thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring =
        depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bone_inputs = bones_from_py(bones)?;
    bridge_chains_in_scope(&spring, &bone_inputs, chain_name, scope)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))
}

#[pyfunction]
pub fn chains_under_bone(
    _py: Python<'_>,
    spring: &Bound<PyAny>,
    bones: &Bound<PyAny>,
    bone_name: &str,
) -> PyResult<Vec<String>> {
    let spring: thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring =
        depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bone_inputs = bones_from_py(bones)?;
    bridge_chains_under_bone(&spring, &bone_inputs, bone_name)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))
}
