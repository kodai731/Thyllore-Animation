use pyo3::prelude::*;
use pythonize::depythonize;

use crate::spring_preview_bridge::{MatrixRows, SpringPreview, SpringPreviewRequest};
use thyllore_ml_api::python::{anyhow_to_pyerr, depythonize_error_to_pyerr, pythonize_value};

#[pyclass(name = "SpringPreview", module = "thyllore_avatar_api")]
pub struct PySpringPreview {
    inner: SpringPreview,
}

#[pymethods]
impl PySpringPreview {
    #[new]
    fn new(request: &Bound<PyAny>) -> PyResult<Self> {
        let request: SpringPreviewRequest =
            depythonize(request).map_err(depythonize_error_to_pyerr)?;
        let inner = SpringPreview::from_request(&request).map_err(anyhow_to_pyerr)?;
        Ok(Self { inner })
    }

    fn reset(&mut self, animated_locals: Vec<MatrixRows>) -> PyResult<()> {
        self.inner.reset(&animated_locals).map_err(anyhow_to_pyerr)
    }

    fn step<'py>(
        &mut self,
        py: Python<'py>,
        animated_locals: Vec<MatrixRows>,
        dt: f32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let rotations = self
            .inner
            .step(&animated_locals, dt)
            .map_err(anyhow_to_pyerr)?;
        pythonize_value(py, &rotations)
    }

    fn tails<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        pythonize_value(py, &self.inner.tails())
    }
}
