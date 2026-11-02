use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pythonize::depythonize;

use super::error::anyhow_to_pyerr;
use crate::vrm_bridge::vrm_write as bridge_vrm_write;
use thyllore_avatar_core::vrm::VrmMeta;

#[pyfunction]
#[pyo3(signature = (glb, sidecar_json, meta, metres_per_gltf_unit = 1.0))]
pub fn vrm_write<'py>(
    py: Python<'py>,
    glb: &[u8],
    sidecar_json: &str,
    meta: &Bound<PyAny>,
    metres_per_gltf_unit: f32,
) -> PyResult<Bound<'py, PyBytes>> {
    let meta: VrmMeta =
        depythonize(meta).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let vrm = bridge_vrm_write(glb, sidecar_json, &meta, metres_per_gltf_unit)
        .map_err(anyhow_to_pyerr)?;
    Ok(PyBytes::new(py, &vrm))
}
