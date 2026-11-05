use pyo3::prelude::*;
use pythonize::depythonize;

use crate::clip_bake_bridge::{bake_clip_file as bridge_bake_clip_file, BakeClipFileRequest};
use thyllore_ml_api::python::{anyhow_to_pyerr, depythonize_error_to_pyerr, pythonize_value};

/// Bakes a `.anim.ron` humanoid clip onto `bones` (with `rest_frame`) at `fps`, as the engine plays a template.
#[pyfunction]
pub fn bake_clip_file<'py>(py: Python<'py>, request: &Bound<PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let req: BakeClipFileRequest = depythonize(request).map_err(depythonize_error_to_pyerr)?;
    let baked = bridge_bake_clip_file(&req).map_err(anyhow_to_pyerr)?;
    pythonize_value(py, &baked)
}
