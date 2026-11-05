use pyo3::prelude::*;
use pythonize::depythonize;

use crate::submesh_bridge::{
    find_hidden_triangles as bridge_find_hidden_triangles,
    guess_submesh_role as bridge_guess_submesh_role,
    hidden_mask_defaults as bridge_hidden_mask_defaults, FindHiddenTrianglesRequest,
};
use thyllore_ml_api::python::{depythonize_error_to_pyerr, pythonize_value};

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
