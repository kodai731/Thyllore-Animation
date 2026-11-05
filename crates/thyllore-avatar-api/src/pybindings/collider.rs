use pyo3::prelude::*;
use pythonize::depythonize;

use crate::collider_bridge::{
    assign_collider_groups as bridge_assign_collider_groups, capsule_fit_params,
    fit_body_colliders as bridge_fit_body_colliders,
    has_fitted_body_colliders as bridge_has_fitted_body_colliders, AssignColliderGroupsRequest,
    FitBodyCollidersRequest, FitBone,
};
use thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring;
use thyllore_ml_api::python::{depythonize_error_to_pyerr, pythonize_value};

#[pyfunction]
pub fn capsule_fit_ui_params(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    pythonize_value(py, &capsule_fit_params())
}

#[pyfunction]
pub fn fit_body_colliders<'py>(
    py: Python<'py>,
    request: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let request: FitBodyCollidersRequest =
        depythonize(request).map_err(depythonize_error_to_pyerr)?;

    let output = py
        .detach(|| bridge_fit_body_colliders(&request))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))?;

    pythonize_value(py, &output)
}

#[pyfunction]
#[pyo3(signature = (spring, bones))]
pub fn has_fitted_body_colliders(spring: &Bound<PyAny>, bones: &Bound<PyAny>) -> PyResult<bool> {
    let spring: SidecarSpring = depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bones: Vec<FitBone> = depythonize(bones).map_err(depythonize_error_to_pyerr)?;

    bridge_has_fitted_body_colliders(&spring, &bones)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))
}

#[pyfunction]
pub fn assign_collider_groups<'py>(
    py: Python<'py>,
    request: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let request: AssignColliderGroupsRequest =
        depythonize(request).map_err(depythonize_error_to_pyerr)?;

    let output = bridge_assign_collider_groups(&request)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))?;

    pythonize_value(py, &output)
}
