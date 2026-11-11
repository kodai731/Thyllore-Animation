use pyo3::prelude::*;
use pythonize::depythonize;

use crate::submesh_bridge::{
    find_hidden_triangles as bridge_find_hidden_triangles,
    guess_submesh_role as bridge_guess_submesh_role,
    hidden_mask_defaults as bridge_hidden_mask_defaults, submesh_roles as bridge_submesh_roles,
    FindHiddenTrianglesRequest,
};
use thyllore_ml_api::python::{anyhow_to_pyerr, depythonize_error_to_pyerr, pythonize_value};

/// Every submesh role with its sidecar `name` and `label`, in declaration order.
#[pyfunction]
pub fn submesh_roles(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    let entries = bridge_submesh_roles().map_err(anyhow_to_pyerr)?;
    pythonize_value(py, &entries)
}

#[pyfunction]
pub fn guess_submesh_role(mesh: &str, material: &str) -> String {
    bridge_guess_submesh_role(mesh, material)
}

#[pyfunction]
pub fn find_hidden_triangles<'py>(
    py: Python<'py>,
    request: &Bound<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let request: FindHiddenTrianglesRequest =
        depythonize(request).map_err(depythonize_error_to_pyerr)?;
    let output = py.detach(|| bridge_find_hidden_triangles(&request));
    pythonize_value(py, &output)
}

#[pyfunction]
pub fn hidden_mask_defaults(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    let defaults = bridge_hidden_mask_defaults();
    pythonize_value(py, &defaults)
}
