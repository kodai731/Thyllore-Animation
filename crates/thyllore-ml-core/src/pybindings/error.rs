use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

pub fn shape_mismatch(field: &str, expected: usize, actual: usize) -> PyErr {
    PyValueError::new_err(format!(
        "{} shape mismatch: expected {} elements, got {}",
        field, expected, actual
    ))
}
