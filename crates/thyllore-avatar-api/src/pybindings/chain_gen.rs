use pyo3::prelude::*;
use pythonize::depythonize;
use thyllore_ml_api::python::{anyhow_to_pyerr, depythonize_error_to_pyerr, pythonize_value};

use crate::chain_gen_bridge::{
    body_colliders as body_colliders_from_inputs, chain_gen_params, role_bones, run_chain_gen,
    settings_from_pairs, submesh_chain_names as submesh_chain_names_of, GenerateChainsRequest,
    SkinAttachment,
};
use thyllore_avatar_core::cloth_setup::components::chain_gen::default_root_of_submesh;
use thyllore_avatar_core::cloth_setup::components::submesh_role::SubmeshRole;

#[derive(serde::Serialize)]
struct ChainRoleEntry {
    role: SubmeshRole,
    default_root: &'static str,
}

#[pyfunction]
pub fn chain_roles(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    let entries: Vec<ChainRoleEntry> = SubmeshRole::ALL
        .into_iter()
        .filter_map(|role| {
            let root = default_root_of_submesh(role)?;
            Some(ChainRoleEntry {
                role,
                default_root: root.unity_name(),
            })
        })
        .collect();
    pythonize_value(py, &entries)
}

/// The chain generation settings table: name, label, min, max, format, tooltip and default per row.
#[pyfunction]
pub fn chain_gen_ui_params(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    pythonize_value(py, &chain_gen_params())
}

/// The names among `candidates` that belong to a chain generated on the submesh `target`
/// (`<mesh>/<material>`); the others name chains of other submeshes.
#[pyfunction]
pub fn submesh_chain_names(target: &str, candidates: Vec<String>) -> PyResult<Vec<String>> {
    submesh_chain_names_of(target, &candidates).map_err(anyhow_to_pyerr)
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
/// chain tail); the chains named for this submesh replace the split.
#[pyfunction]
pub fn generate_chains<'py>(
    py: Python<'py>,
    request: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let request: GenerateChainsRequest =
        depythonize(request).map_err(depythonize_error_to_pyerr)?;

    let settings = settings_from_pairs(
        &request
            .settings
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect::<Vec<_>>(),
    )
    .map_err(anyhow_to_pyerr)?;
    let roles = role_bones(&request.humanoid_bones, &request.bones);
    let colliders = body_colliders_from_inputs(&request.body_colliders, &roles, &request.bones);

    let output = py
        .detach(|| {
            run_chain_gen(
                &request.positions,
                &request.triangles,
                request.root_position,
                &request.target,
                settings,
                SkinAttachment {
                    skins: &request.skins,
                    body_colliders: &colliders,
                    roles: &roles,
                    root_bone: request.root_bone,
                    covered_vertices: &request.covered_vertices,
                },
                request.attachment,
                &request.tex_coords,
                &request.given_chains,
            )
        })
        .map_err(anyhow_to_pyerr)?;

    pythonize_value(py, &output)
}
