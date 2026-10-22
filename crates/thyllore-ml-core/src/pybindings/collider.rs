use pyo3::prelude::*;
use pyo3::types::PyDict;
use pythonize::depythonize;

use super::avatar_setup::{bones_from_py, depythonize_error_to_pyerr};
use super::chain_gen::{pythonize_value, settings_pairs};
use crate::collider_bridge::{
    assign_collider_groups as bridge_assign_collider_groups, capsule_fit_params,
    fit_body_colliders as bridge_fit_body_colliders, FitBone, FitSkin,
};
use thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring;

#[pyfunction]
pub fn capsule_fit_ui_params(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    pythonize_value(py, &capsule_fit_params())
}

#[pyfunction]
#[pyo3(signature = (spring, bones, skins, settings))]
pub fn fit_body_colliders<'py>(
    py: Python<'py>,
    spring: &Bound<PyAny>,
    bones: &Bound<PyAny>,
    skins: &Bound<PyAny>,
    settings: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let spring: SidecarSpring = depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bones: Vec<FitBone> = depythonize(bones).map_err(depythonize_error_to_pyerr)?;
    let skins: Vec<FitSkin> = depythonize(skins).map_err(depythonize_error_to_pyerr)?;
    let pairs = settings_pairs(settings)?;

    let output = py
        .detach(|| bridge_fit_body_colliders(&spring, &bones, &skins, &pairs))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))?;

    pythonize_value(py, &output)
}

#[pyfunction]
#[pyo3(signature = (spring, bones, chain_names, group_names))]
pub fn assign_collider_groups<'py>(
    py: Python<'py>,
    spring: &Bound<PyAny>,
    bones: &Bound<PyAny>,
    chain_names: Vec<String>,
    group_names: Vec<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let spring: SidecarSpring = depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bone_inputs = bones_from_py(bones)?;

    let output = bridge_assign_collider_groups(&spring, &bone_inputs, &chain_names, &group_names)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))?;

    pythonize_value(py, &output)
}
