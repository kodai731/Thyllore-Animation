use pyo3::prelude::*;
use pyo3::types::PyDict;
use pythonize::{depythonize, pythonize};

use super::avatar_setup::depythonize_error_to_pyerr;
use super::error::{anyhow_to_pyerr, anyhow_to_value_error};
use std::collections::BTreeMap;

use crate::avatar_setup_bridge::BoneSpec;
use crate::chain_gen_bridge::{
    body_colliders as body_colliders_from_inputs, chain_gen_params,
    legacy_chain_renames as legacy_chain_renames_of, parse_attachment, role_bones, run_chain_gen,
    settings_from_pairs, submesh_chain_names as submesh_chain_names_of, BodyColliderIn,
    ChainRootIn, GivenChainIn, SkinAttachment,
};
use crate::collider_bridge::FitSkin;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

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

/// The names among `candidates` that belong to a chain generated on the submesh `target`
/// (`<mesh>/<material>`); the others name chains of other submeshes.
#[pyfunction]
pub fn submesh_chain_names(target: &str, candidates: Vec<String>) -> PyResult<Vec<String>> {
    submesh_chain_names_of(target, &candidates).map_err(anyhow_to_pyerr)
}

/// `(old name, new name)` for the candidates that name a chain of the submesh `target` under the
/// legacy `<role>_<label>_<column>` scheme; renaming them keeps the chains a regenerate reads back.
#[pyfunction]
pub fn legacy_chain_renames(
    target: &str,
    candidates: Vec<String>,
) -> PyResult<Vec<(String, String)>> {
    legacy_chain_renames_of(target, &candidates).map_err(anyhow_to_pyerr)
}

fn parse_root_role(root_role: Option<&str>) -> PyResult<Option<HumanoidRole>> {
    root_role
        .map(|name| {
            HumanoidRole::from_unity_name(name).ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "root_role must be a humanoid role name such as Head or Hips, got \"{name}\""
                ))
            })
        })
        .transpose()
}

/// Positions, the root position and the skins are engine coordinates (Y-up, metres); `skins` are the
/// FitSkin dicts (positions, influences, triangles) and `root_bone` the index of the bone the chains hang
/// from; `target` is `<mesh>/<material>` and names the chains; `settings` maps setting names to values;
/// `tex_coords` is per-vertex UV, empty uses the φ-around-up method; `covered_vertices` are the garment
/// vertices the user marked as covered by the body, empty finds the rings the body supports instead;
/// `body_colliders` are the sidecar colliders in model axes relative to their bones' rest heads (bone
/// index, head, tail, radius), `humanoid_bones` maps Unity role names to bone indices and `bones` are
/// the bone specs; together they give the torso volume the garment can rest on. `given_chains` are the
/// chains the user assigned as `{name, vertices, joints?}` dicts (`joints` are the joint heads followed by the
/// chain tail); the chains named for this submesh replace the split. `root_role` is the Unity role name
/// the root was chosen by, when it was; a root given as a bone takes its role from `humanoid_bones`.
/// `attachment` is `"cap"` or `"ring"`; absent, it follows the root role.
#[pyfunction]
#[pyo3(signature = (positions, triangles, root_position, target, settings, skins = None, root_bone = 0, tex_coords = Vec::new(), covered_vertices = Vec::new(), body_colliders = None, humanoid_bones = BTreeMap::new(), bones = None, given_chains = None, *, root_role = None, attachment = None))]
#[allow(clippy::too_many_arguments)]
pub fn generate_chains<'py>(
    py: Python<'py>,
    positions: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    root_position: [f32; 3],
    target: &str,
    settings: &Bound<'py, PyDict>,
    skins: Option<&Bound<'py, PyAny>>,
    root_bone: usize,
    tex_coords: Vec<[f32; 2]>,
    covered_vertices: Vec<u32>,
    body_colliders: Option<&Bound<'py, PyAny>>,
    humanoid_bones: BTreeMap<String, usize>,
    bones: Option<&Bound<'py, PyAny>>,
    given_chains: Option<&Bound<'py, PyAny>>,
    root_role: Option<&str>,
    attachment: Option<&str>,
) -> PyResult<Bound<'py, PyAny>> {
    let pairs = settings_pairs(settings)?;
    let settings = settings_from_pairs(&pairs).map_err(anyhow_to_pyerr)?;
    let skins: Vec<FitSkin> = match skins {
        Some(skins) => depythonize(skins).map_err(depythonize_error_to_pyerr)?,
        None => Vec::new(),
    };
    let collider_inputs: Vec<BodyColliderIn> = match body_colliders {
        Some(colliders) => depythonize(colliders).map_err(depythonize_error_to_pyerr)?,
        None => Vec::new(),
    };
    let bone_specs: Vec<BoneSpec> = match bones {
        Some(bones) => depythonize(bones).map_err(depythonize_error_to_pyerr)?,
        None => Vec::new(),
    };
    let given: Vec<GivenChainIn> = match given_chains {
        Some(given) => depythonize(given).map_err(depythonize_error_to_pyerr)?,
        None => Vec::new(),
    };
    let roles = role_bones(&humanoid_bones, &bone_specs);
    let colliders = body_colliders_from_inputs(&collider_inputs, &roles, &bone_specs);

    let chain_root = ChainRootIn {
        bone: root_bone,
        role: parse_root_role(root_role)?,
        attachment: attachment
            .map(parse_attachment)
            .transpose()
            .map_err(anyhow_to_value_error)?,
    };

    let output = py
        .detach(|| {
            run_chain_gen(
                &positions,
                &triangles,
                root_position,
                target,
                settings,
                SkinAttachment {
                    skins: &skins,
                    body_colliders: &colliders,
                    roles: &roles,
                    covered_vertices: &covered_vertices,
                },
                chain_root,
                &tex_coords,
                &given,
            )
        })
        .map_err(anyhow_to_pyerr)?;

    pythonize_value(py, &output)
}
