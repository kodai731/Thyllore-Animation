use pyo3::prelude::*;
use pyo3::types::PyDict;
use pythonize::{depythonize, pythonize};

use super::avatar_setup::depythonize_error_to_pyerr;
use super::error::anyhow_to_pyerr;
use crate::chain_gen_bridge::{
    chain_gen_params, run_chain_gen, settings_from_pairs, SkinAttachment,
};
use crate::collider_bridge::FitSkin;

pub(super) fn pythonize_value<'py, T: serde::Serialize>(
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

pub(super) fn settings_pairs(settings: &Bound<PyDict>) -> PyResult<Vec<(String, f32)>> {
    settings
        .iter()
        .map(|(key, value)| Ok((key.extract::<String>()?, value.extract::<f32>()?)))
        .collect::<PyResult<_>>()
}

/// Positions, the root position and the skins are engine coordinates (Y-up, metres); `skins` are the
/// FitSkin dicts (positions, influences, triangles) and `root_bone` the index of the bone the chains hang
/// from; `target` is `<mesh>/<material>` and names the chains; `settings` maps setting names to values;
/// `tex_coords` is per-vertex UV, empty uses the φ-around-up method; `covered_vertices` are the garment
/// vertices the user marked as covered by the skin, empty measures the coverage by distance.
#[pyfunction]
#[pyo3(signature = (positions, triangles, root_position, role, target, settings, skins = None, root_bone = 0, tex_coords = Vec::new(), covered_vertices = Vec::new()))]
#[allow(clippy::too_many_arguments)]
pub fn generate_chains<'py>(
    py: Python<'py>,
    positions: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    root_position: [f32; 3],
    role: &str,
    target: &str,
    settings: &Bound<'py, PyDict>,
    skins: Option<&Bound<'py, PyAny>>,
    root_bone: usize,
    tex_coords: Vec<[f32; 2]>,
    covered_vertices: Vec<u32>,
) -> PyResult<Bound<'py, PyAny>> {
    let pairs = settings_pairs(settings)?;
    let settings = settings_from_pairs(&pairs).map_err(anyhow_to_pyerr)?;
    let skins: Vec<FitSkin> = match skins {
        Some(skins) => depythonize(skins).map_err(depythonize_error_to_pyerr)?,
        None => Vec::new(),
    };

    let output = py
        .detach(|| {
            run_chain_gen(
                &positions,
                &triangles,
                root_position,
                role,
                target,
                settings,
                SkinAttachment {
                    skins: &skins,
                    root_bone,
                    covered_vertices: &covered_vertices,
                },
                &tex_coords,
            )
        })
        .map_err(anyhow_to_pyerr)?;

    pythonize_value(py, &output)
}
