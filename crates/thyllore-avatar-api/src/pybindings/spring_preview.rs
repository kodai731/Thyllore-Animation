use pyo3::prelude::*;
use pythonize::depythonize;

use crate::spring_preview_bridge::{PoseRequest, SpringPreview, SpringPreviewRequest, StepRequest};
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
        let inner = SpringPreview::new(&request).map_err(anyhow_to_pyerr)?;
        Ok(Self { inner })
    }

    fn reset(&mut self, request: &Bound<PyAny>) -> PyResult<()> {
        let request: PoseRequest = depythonize(request).map_err(depythonize_error_to_pyerr)?;
        self.inner.reset(&request).map_err(anyhow_to_pyerr)
    }

    fn step<'py>(
        &mut self,
        py: Python<'py>,
        request: &Bound<PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let request: StepRequest = depythonize(request).map_err(depythonize_error_to_pyerr)?;
        let rotations = self.inner.step(&request).map_err(anyhow_to_pyerr)?;
        pythonize_value(py, &rotations)
    }

    fn tails<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        pythonize_value(py, &self.inner.tails())
    }
}
