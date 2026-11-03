use pyo3::prelude::*;
use pythonize::depythonize;

use super::avatar_setup::{bones_from_py, depythonize_error_to_pyerr};
use super::chain_gen::pythonize_value;
use crate::chain_gen_bridge::CapsuleOut;
use crate::chain_register_bridge::{
    register_generated_chains as bridge_register_generated_chains, GeneratedChainIn,
    RegisterRequest,
};
use crate::collider_bridge::{FitBone, FitSkin};
use crate::spring_edit_bridge::{
    add_chains_under_bone as bridge_add_chains_under_bone, apply_ramp_to_chains,
    chains_in_scope as bridge_chains_in_scope, chains_under_bone as bridge_chains_under_bone,
    chains_with_radius as bridge_chains_with_radius, resolve_root_bone_name, RampInput,
};

/// Adds a chain for every run of non-humanoid bones under `root_bone` with at least `min_joints` bones.
#[pyfunction]
#[pyo3(signature = (spring, bones, root_bone, min_joints = 2))]
pub fn add_chains_under_bone<'py>(
    py: Python<'py>,
    spring: &Bound<PyAny>,
    bones: &Bound<PyAny>,
    root_bone: &str,
    min_joints: u32,
) -> PyResult<Bound<'py, PyAny>> {
    let spring: thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring =
        depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bone_inputs = bones_from_py(bones)?;
    let output = bridge_add_chains_under_bone(&spring, &bone_inputs, root_bone, min_joints)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))?;
    pythonize_value(py, &output)
}

/// Registers the chains of a `generate_chains` output as the engine does: joint radii, the head
/// capsule (or a capsule fitted to the skin on the root bone) in the fitted body group, and the
/// chains bound to that group. `bones` carry `bind_matrix` like the collider fit.
#[pyfunction]
#[pyo3(signature = (spring, bones, skins, root_bone, target, role, chains, head_capsule = None))]
#[allow(clippy::too_many_arguments)]
pub fn register_generated_chains<'py>(
    py: Python<'py>,
    spring: &Bound<PyAny>,
    bones: &Bound<PyAny>,
    skins: &Bound<PyAny>,
    root_bone: &str,
    target: &str,
    role: &str,
    chains: &Bound<PyAny>,
    head_capsule: Option<&Bound<PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let spring: thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring =
        depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bones: Vec<FitBone> = depythonize(bones).map_err(depythonize_error_to_pyerr)?;
    let skins: Vec<FitSkin> = depythonize(skins).map_err(depythonize_error_to_pyerr)?;
    let chains: Vec<GeneratedChainIn> = depythonize(chains).map_err(depythonize_error_to_pyerr)?;
    let head_capsule: Option<CapsuleOut> = head_capsule
        .map(|capsule| depythonize(capsule).map_err(depythonize_error_to_pyerr))
        .transpose()?;
    let request = RegisterRequest {
        root_name: root_bone,
        target,
        role,
        chains: &chains,
        head_capsule,
    };
    let output = bridge_register_generated_chains(&spring, &bones, &skins, &request)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))?;
    pythonize_value(py, &output)
}

#[pyfunction]
pub fn apply_ramp<'py>(
    py: Python<'py>,
    spring: &Bound<PyAny>,
    bones: &Bound<PyAny>,
    chain_names: Vec<String>,
    ramp: &Bound<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let spring: thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring =
        depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bone_inputs = bones_from_py(bones)?;
    let ramp_input: RampInput = depythonize(ramp).map_err(depythonize_error_to_pyerr)?;
    let output = apply_ramp_to_chains(&spring, &bone_inputs, &chain_names, &ramp_input)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))?;
    pythonize_value(py, &output)
}

#[pyfunction]
pub fn chains_in_scope(
    spring: &Bound<PyAny>,
    bones: &Bound<PyAny>,
    chain_name: &str,
    scope: &str,
) -> PyResult<Vec<String>> {
    let spring: thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring =
        depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bone_inputs = bones_from_py(bones)?;
    bridge_chains_in_scope(&spring, &bone_inputs, chain_name, scope)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))
}

#[pyfunction]
pub fn chains_under_bone(
    spring: &Bound<PyAny>,
    bones: &Bound<PyAny>,
    bone_name: &str,
) -> PyResult<Vec<String>> {
    let spring: thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring =
        depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bone_inputs = bones_from_py(bones)?;
    bridge_chains_under_bone(&spring, &bone_inputs, bone_name)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))
}

#[pyfunction]
pub fn chains_with_radius(
    spring: &Bound<PyAny>,
    bones: &Bound<PyAny>,
    active: Option<usize>,
    mode: &str,
) -> PyResult<Vec<String>> {
    let spring: thyllore_avatar_core::vrchat::sidecar_spring::SidecarSpring =
        depythonize(spring).map_err(depythonize_error_to_pyerr)?;
    let bone_inputs = bones_from_py(bones)?;
    bridge_chains_with_radius(&spring, &bone_inputs, active, mode)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))
}

#[pyfunction]
pub fn spring_gizmo_style<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    use thyllore_anim_core::spring_bone::{
        CHAIN_RADIUS_SEGMENTS, CIRCLE_SEGMENTS, COLOR_CHAIN, COLOR_COLLIDER,
    };
    let style = SpringGizmoStyle {
        chain_color: COLOR_CHAIN,
        collider_color: COLOR_COLLIDER,
        circle_segments: CIRCLE_SEGMENTS,
        chain_radius_segments: CHAIN_RADIUS_SEGMENTS,
    };
    pythonize_value(py, &style)
}

#[derive(serde::Serialize)]
struct SpringGizmoStyle {
    chain_color: [f32; 3],
    collider_color: [f32; 3],
    circle_segments: u32,
    chain_radius_segments: u32,
}

#[pyfunction]
pub fn resolve_root_bone(bones: &Bound<PyAny>, root_bone: &str) -> PyResult<String> {
    let bone_inputs = bones_from_py(bones)?;
    resolve_root_bone_name(&bone_inputs, root_bone)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:#}", e)))
}
