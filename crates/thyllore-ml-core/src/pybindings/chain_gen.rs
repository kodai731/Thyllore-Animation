use pyo3::prelude::*;
use pythonize::pythonize;

use super::error::anyhow_to_pyerr;
use crate::chain_gen_bridge::run_chain_gen;

#[pyfunction]
#[pyo3(signature = (positions, triangles, root_position, up, role, joint_count, column_count))]
pub fn generate_chains<'py>(
    py: Python<'py>,
    positions: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    root_position: [f32; 3],
    up: [f32; 3],
    role: &str,
    joint_count: usize,
    column_count: usize,
) -> PyResult<Bound<'py, PyAny>> {
    let output = py
        .detach(|| {
            run_chain_gen(
                &positions,
                &triangles,
                root_position,
                up,
                role,
                joint_count,
                column_count,
            )
        })
        .map_err(anyhow_to_pyerr)?;

    pythonize(py, &output).map_err(|e: pythonize::PythonizeError| {
        pyo3::exceptions::PyValueError::new_err(e.to_string())
    })
}
