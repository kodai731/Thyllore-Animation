use pyo3::prelude::*;
use pyo3::types::PyDict;
use pythonize::pythonize;

use super::error::anyhow_to_pyerr;
use crate::chain_gen_bridge::{chain_gen_params, run_chain_gen, settings_from_pairs};

fn pythonize_value<'py, T: serde::Serialize>(
    py: Python<'py>,
    value: &T,
) -> PyResult<Bound<'py, PyAny>> {
    pythonize(py, value).map_err(|e: pythonize::PythonizeError| {
        pyo3::exceptions::PyValueError::new_err(e.to_string())
    })
}

/// The chain generation settings table: name, label, min, max, format, tooltip and default per row.
#[pyfunction]
pub fn chain_gen_ui_params(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    pythonize_value(py, &chain_gen_params())
}

/// Positions, the root position and the attachment (skin) positions are engine coordinates (Y-up, metres);
/// `settings` maps setting names to values.
#[pyfunction]
#[pyo3(signature = (positions, triangles, root_position, role, settings, attachment_positions = Vec::new()))]
pub fn generate_chains<'py>(
    py: Python<'py>,
    positions: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    root_position: [f32; 3],
    role: &str,
    settings: &Bound<'py, PyDict>,
    attachment_positions: Vec<[f32; 3]>,
) -> PyResult<Bound<'py, PyAny>> {
    let pairs: Vec<(String, f32)> = settings
        .iter()
        .map(|(key, value)| Ok((key.extract::<String>()?, value.extract::<f32>()?)))
        .collect::<PyResult<_>>()?;
    let settings = settings_from_pairs(&pairs).map_err(anyhow_to_pyerr)?;

    let output = py
        .detach(|| {
            run_chain_gen(
                &positions,
                &triangles,
                root_position,
                role,
                settings,
                &attachment_positions,
            )
        })
        .map_err(anyhow_to_pyerr)?;

    pythonize_value(py, &output)
}
