use pyo3::prelude::*;

mod avatar_setup;
mod chain_gen;
mod clip_bake;
mod collider;
mod gizmo;
mod spring_edit;
mod spring_preview;
mod submesh;
mod vrm;

pub use spring_preview::PySpringPreview;

#[pymodule]
fn thyllore_avatar_api(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__abi_marker__", thyllore_ml_api::ABI_MARKER)?;

    m.add_class::<PySpringPreview>()?;

    m.add_function(wrap_pyfunction!(chain_gen::generate_chains, m)?)?;
    m.add_function(wrap_pyfunction!(chain_gen::chain_gen_ui_params, m)?)?;
    m.add_function(wrap_pyfunction!(chain_gen::submesh_chain_names, m)?)?;
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
    m.add_function(wrap_pyfunction!(spring_edit::add_chains_under_bone, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::register_generated_chains, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::apply_ramp, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::chains_in_scope, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::chains_under_bone, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::chains_with_radius, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::resolve_root_bone, m)?)?;
    m.add_function(wrap_pyfunction!(spring_edit::spring_gizmo_style, m)?)?;
    m.add_function(wrap_pyfunction!(gizmo::wire_sphere_segments, m)?)?;
    m.add_function(wrap_pyfunction!(gizmo::wire_capsule_segments, m)?)?;
    m.add_function(wrap_pyfunction!(collider::capsule_fit_ui_params, m)?)?;
    m.add_function(wrap_pyfunction!(collider::fit_body_colliders, m)?)?;
    m.add_function(wrap_pyfunction!(collider::has_fitted_body_colliders, m)?)?;
    m.add_function(wrap_pyfunction!(collider::assign_collider_groups, m)?)?;
    m.add_function(wrap_pyfunction!(submesh::guess_submesh_role, m)?)?;
    m.add_function(wrap_pyfunction!(submesh::find_hidden_triangles, m)?)?;
    m.add_function(wrap_pyfunction!(submesh::hidden_mask_defaults, m)?)?;
    m.add_function(wrap_pyfunction!(vrm::vrm_write, m)?)?;
    m.add_function(wrap_pyfunction!(vrm::vrm_meta_fields, m)?)?;
    m.add_function(wrap_pyfunction!(vrm::vrm_t_pose_warnings, m)?)?;

    Ok(())
}
