use pyo3::prelude::*;
use pythonize::depythonize;

use crate::spring_preview_bridge::{MatrixRows, SpringPreview};

use super::avatar_setup::{bones_from_py, depythonize_error_to_pyerr};
use super::chain_gen::pythonize_value;
use super::error::anyhow_to_pyerr;

#[pyclass(name = "SpringPreview", module = "thyllore_ml_core")]
pub struct PySpringPreview {
    inner: SpringPreview,
}

#[pymethods]
impl PySpringPreview {
    #[new]
    fn new(
        spring: &Bound<PyAny>,
        bones: &Bound<PyAny>,
        rest_locals: Vec<MatrixRows>,
    ) -> PyResult<Self> {
        let spring: thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring =
            depythonize(spring).map_err(depythonize_error_to_pyerr)?;
        let bone_inputs = bones_from_py(bones)?;
        let inner =
            SpringPreview::new(&spring, &bone_inputs, &rest_locals).map_err(anyhow_to_pyerr)?;
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
