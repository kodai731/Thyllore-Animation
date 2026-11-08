use pyo3::prelude::*;

use super::*;

#[pymodule]
fn thyllore_avatar_api(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__abi_marker__", thyllore_ml_api::ABI_MARKER)?;

    m.add_class::<PySpringPreview>()?;

    m.add_function(wrap_pyfunction!(chain_gen::generate_chains, m)?)?;
    m.add_function(wrap_pyfunction!(chain_gen::chain_roles, m)?)?;
    m.add_function(wrap_pyfunction!(chain_gen::root_attachments, m)?)?;
    m.add_function(wrap_pyfunction!(chain_gen::chain_bone_palette, m)?)?;
    m.add_function(wrap_pyfunction!(chain_gen::vertex_group_names, m)?)?;
    m.add_function(wrap_pyfunction!(chain_gen::membership_vertex_group, m)?)?;
    m.add_function(wrap_pyfunction!(chain_gen::chain_of_vertex_group, m)?)?;
    m.add_function(wrap_pyfunction!(chain_gen::chain_gen_ui_params, m)?)?;
    m.add_function(wrap_pyfunction!(chain_gen::submesh_chain_names, m)?)?;
    m.add_function(wrap_pyfunction!(chain_gen::legacy_chain_renames, m)?)?;
    m.add_function(wrap_pyfunction!(clip_bake::bake_clip_file, m)?)?;
    m.add_function(wrap_pyfunction!(avatar_setup::spring_joint_ui_params, m)?)?;
    m.add_function(wrap_pyfunction!(
        avatar_setup::spring_joint_force_ui_params,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(avatar_setup::sidecar_read, m)?)?;
    m.add_function(wrap_pyfunction!(avatar_setup::sidecar_build, m)?)?;
    m.add_function(wrap_pyfunction!(
        avatar_setup::spring_colliders_to_local,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        avatar_setup::spring_colliders_to_model,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(avatar_setup::infer_humanoid_mapping, m)?)?;
    m.add_function(wrap_pyfunction!(
        spring_edit::use_existing_bones_as_springs,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(spring_edit::register_generated_chains, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::apply_ramp, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::chains_in_scope, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::chains_under_bone, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::chains_with_radius, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::resolve_root_bone, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::spring_gizmo_style, m)?)?;
    m.add_function(wrap_pyfunction!(gizmo::wire_sphere_segments, m)?)?;
    m.add_function(wrap_pyfunction!(gizmo::wire_capsule_segments, m)?)?;
    m.add_function(wrap_pyfunction!(blender_space::armature_local_points, m)?)?;
    m.add_function(wrap_pyfunction!(blender_space::bone_specs, m)?)?;
    m.add_function(wrap_pyfunction!(collider::capsule_fit_ui_params, m)?)?;
    m.add_function(wrap_pyfunction!(collider::fit_body_colliders, m)?)?;
    m.add_function(wrap_pyfunction!(collider::has_fitted_body_colliders, m)?)?;
    m.add_function(wrap_pyfunction!(collider::assign_collider_groups, m)?)?;
    m.add_function(wrap_pyfunction!(submesh::guess_submesh_role, m)?)?;
    m.add_function(wrap_pyfunction!(submesh::find_hidden_triangles, m)?)?;
    m.add_function(wrap_pyfunction!(submesh::hidden_mask_defaults, m)?)?;
    m.add_function(wrap_pyfunction!(skin_weight::transfer_skin_weights, m)?)?;
    m.add_function(wrap_pyfunction!(vrm::vrm_write, m)?)?;
    m.add_function(wrap_pyfunction!(vrm::vrm_meta_fields, m)?)?;
    m.add_function(wrap_pyfunction!(vrm::vrm_t_pose_warnings, m)?)?;
    m.add_function(wrap_pyfunction!(vrm::vrm_rest_clearance_warnings, m)?)?;

    #[cfg(feature = "debug-log")]
    thyllore_ml_api::debug_log::register(m)?;

    Ok(())
}
