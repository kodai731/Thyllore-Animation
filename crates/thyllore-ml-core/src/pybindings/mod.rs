use pyo3::prelude::*;

mod avatar_setup;
mod chain_gen;
mod clip_bake;
mod collider;
#[cfg(feature = "debug-log")]
mod debuglog;
mod error;
mod feedback;
mod spring_edit;
mod spring_preview;
mod submesh;
mod v2_curve_copilot;

pub use spring_preview::PySpringPreview;
pub use v2_curve_copilot::PyV2CurveCopilotSession;

#[pymodule]
fn thyllore_ml_core(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__abi_marker__", thyllore_ml_api::ABI_MARKER)?;

    m.add_class::<PyV2CurveCopilotSession>()?;
    m.add_class::<PySpringPreview>()?;

    m.add_function(wrap_pyfunction!(v2_curve_copilot::capabilities, m)?)?;
    m.add_function(wrap_pyfunction!(v2_curve_copilot::deploy_fps, m)?)?;
    m.add_function(wrap_pyfunction!(
        v2_curve_copilot::resolve_curve_copilot_model_path,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        v2_curve_copilot::forecast_sample_offsets,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(v2_curve_copilot::resolve_origin_frame, m)?)?;
    m.add_function(wrap_pyfunction!(
        v2_curve_copilot::effective_context_length,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        v2_curve_copilot::degraded_context_length,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(chain_gen::generate_chains, m)?)?;
    m.add_function(wrap_pyfunction!(chain_gen::chain_gen_ui_params, m)?)?;
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
    m.add_function(wrap_pyfunction!(collider::capsule_fit_ui_params, m)?)?;
    m.add_function(wrap_pyfunction!(collider::fit_body_colliders, m)?)?;
    m.add_function(wrap_pyfunction!(collider::assign_collider_groups, m)?)?;
    m.add_function(wrap_pyfunction!(feedback::build_feedback_record, m)?)?;
    m.add_function(wrap_pyfunction!(feedback::send_feedback_batch, m)?)?;
    m.add_function(wrap_pyfunction!(feedback::send_message, m)?)?;
    m.add_function(wrap_pyfunction!(submesh::guess_submesh_role, m)?)?;
    m.add_function(wrap_pyfunction!(submesh::find_hidden_triangles, m)?)?;
    m.add_function(wrap_pyfunction!(submesh::hidden_mask_defaults, m)?)?;

    #[cfg(feature = "debug-log")]
    {
        m.add_function(wrap_pyfunction!(debuglog::debug_log_init, m)?)?;
        m.add_function(wrap_pyfunction!(debuglog::debug_log, m)?)?;
    }

    Ok(())
}
