use pyo3::prelude::*;
use pythonize::depythonize;

use super::avatar_setup::depythonize_error_to_pyerr;
use super::chain_gen::pythonize_value;
use crate::submesh_bridge::{
    find_hidden_triangles as bridge_find_hidden_triangles,
    guess_submesh_role as bridge_guess_submesh_role,
    hidden_mask_defaults as bridge_hidden_mask_defaults, MaskMeshInput,
};
use thyllore_avatar_core::cloth_setup::components::hidden_mask::HiddenMaskParams;

#[pyfunction]
pub fn guess_submesh_role(mesh: &str, material: &str) -> String {
    bridge_guess_submesh_role(mesh, material)
}

#[pyfunction]
#[pyo3(signature = (body, garments, params))]
pub fn find_hidden_triangles(
    py: Python<'_>,
    body: &Bound<PyAny>,
    garments: Vec<Bound<'_, PyAny>>,
    params: &Bound<PyAny>,
) -> PyResult<Vec<u32>> {
    let body_input: MaskMeshInput = depythonize(body).map_err(depythonize_error_to_pyerr)?;
    let params: HiddenMaskParams = depythonize(params).map_err(depythonize_error_to_pyerr)?;
    let garment_inputs: Vec<MaskMeshInput> = garments
        .iter()
        .map(|g| depythonize(&g).map_err(depythonize_error_to_pyerr))
        .collect::<PyResult<_>>()?;

    let output = py.detach(|| bridge_find_hidden_triangles(&body_input, &garment_inputs, &params));

    Ok(output)
}

#[pyfunction]
pub fn hidden_mask_defaults(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    let defaults = bridge_hidden_mask_defaults();
    pythonize_value(py, &defaults)
}
