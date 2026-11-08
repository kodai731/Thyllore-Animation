use pyo3::prelude::*;
use pythonize::depythonize;

use super::avatar_setup::bones_from_py;
use crate::chain_register_bridge::{self, RegisterGeneratedChainsRequest};
use crate::spring_edit_bridge::{
    apply_ramp_to_chains, chains_in_scope as bridge_chains_in_scope,
    chains_under_bone as bridge_chains_under_bone, chains_with_radius as bridge_chains_with_radius,
    resolve_root_bone_name, use_existing_bones_as_springs as bridge_use_existing_bones_as_springs,
    ApplyRampRequest, ChainsInScopeRequest, ChainsUnderBoneRequest, ChainsWithRadiusRequest,
    UseExistingBonesAsSpringsRequest,
};
use thyllore_ml_api::python::{depythonize_error_to_pyerr, pythonize_value};

/// Registers every run of existing non-humanoid bones under `root_bone` with at least `min_joints` bones as a spring chain; adds no bones.
#[pyfunction]
pub fn use_existing_bones_as_springs<'py>(
    py: Python<'py>,
    request: &Bound<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let request: UseExistingBonesAsSpringsRequest =
        depythonize(request).map_err(depythonize_error_to_pyerr)?;
    let output = bridge_use_existing_bones_as_springs(&request)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))?;
    pythonize_value(py, &output)
}

/// Registers the chains of a `generate_chains` output as the engine does: joint radii, the head
/// capsule (or a capsule fitted to the skin on the root bone) in the fitted body group, and the
/// chains bound to that group. `bones` carry `bind_matrix` like the collider fit.
#[pyfunction]
pub fn register_generated_chains<'py>(
    py: Python<'py>,
    request: &Bound<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let request: RegisterGeneratedChainsRequest =
        depythonize(request).map_err(depythonize_error_to_pyerr)?;
    let output = chain_register_bridge::register_generated_chains(&request)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))?;
    pythonize_value(py, &output)
}

#[pyfunction]
pub fn apply_ramp<'py>(py: Python<'py>, request: &Bound<PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let request: ApplyRampRequest = depythonize(request).map_err(depythonize_error_to_pyerr)?;
    let output = apply_ramp_to_chains(&request)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))?;
    pythonize_value(py, &output)
}

#[pyfunction]
pub fn chains_in_scope(request: &Bound<PyAny>) -> PyResult<Vec<String>> {
    let request: ChainsInScopeRequest = depythonize(request).map_err(depythonize_error_to_pyerr)?;
    bridge_chains_in_scope(&request)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))
}

#[pyfunction]
pub fn chains_under_bone(request: &Bound<PyAny>) -> PyResult<Vec<String>> {
    let request: ChainsUnderBoneRequest =
        depythonize(request).map_err(depythonize_error_to_pyerr)?;
    bridge_chains_under_bone(&request)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))
}

#[pyfunction]
pub fn chains_with_radius(request: &Bound<PyAny>) -> PyResult<Vec<String>> {
    let request: ChainsWithRadiusRequest =
        depythonize(request).map_err(depythonize_error_to_pyerr)?;
    bridge_chains_with_radius(&request)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))
}

#[pyfunction]
pub fn spring_gizmo_style<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    use thyllore_anim_core::spring_bone::{
        CHAIN_RADIUS_SEGMENTS, CIRCLE_SEGMENTS, COLOR_CHAIN, COLOR_COLLIDER, COLOR_INACTIVE_ALPHA,
    };
    let style = SpringGizmoStyle {
        chain_color: COLOR_CHAIN,
        collider_color: COLOR_COLLIDER,
        circle_segments: CIRCLE_SEGMENTS,
        chain_radius_segments: CHAIN_RADIUS_SEGMENTS,
        inactive_alpha: COLOR_INACTIVE_ALPHA,
    };
    pythonize_value(py, &style)
}

#[derive(serde::Serialize)]
struct SpringGizmoStyle {
    chain_color: [f32; 3],
    collider_color: [f32; 3],
    circle_segments: u32,
    chain_radius_segments: u32,
    inactive_alpha: f32,
}

#[pyfunction]
pub fn resolve_root_bone(bones: &Bound<PyAny>, root_bone: &str) -> PyResult<String> {
    let bone_inputs = bones_from_py(bones)?;
    resolve_root_bone_name(&bone_inputs, root_bone)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))
}
