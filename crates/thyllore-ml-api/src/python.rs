//! Shared PyO3 helper functions used by both thyllore-ml-core and thyllore-avatar-api wheels.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

pub fn anyhow_to_pyerr(err: anyhow::Error) -> PyErr {
    PyRuntimeError::new_err(format!("{:#}", err))
}

pub fn pythonize_value<'py, T: serde::Serialize>(
    py: Python<'py>,
    value: &T,
) -> PyResult<Bound<'py, PyAny>> {
    pythonize::pythonize(py, value)
        .map_err(|e: pythonize::PythonizeError| PyValueError::new_err(e.to_string()))
}

pub fn depythonize_error_to_pyerr(error: pythonize::PythonizeError) -> PyErr {
    PyValueError::new_err(error.to_string())
}
