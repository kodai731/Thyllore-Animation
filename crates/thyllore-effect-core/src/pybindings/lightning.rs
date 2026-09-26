use super::effect::{
    build_effect_from_params, declare_effect_pyfunctions, gpu_block_bytes,
    inverse_view_proj_from_column_major, ParameterOwnerName,
};
use crate::lightning::{
    build_lightning_ubo, LightningEffect, LightningParameterOwner, LightningSegmentsUBO,
    LightningUBO, LIGHTNING_MAX_WAYPOINTS,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;

impl ParameterOwnerName for LightningParameterOwner {
    fn owner_name(self) -> &'static str {
        "frame"
    }
}

declare_effect_pyfunctions! {
    effect: LightningEffect,
    preset_names: lightning_preset_names,
    ui_params: lightning_ui_params,
    preset_params: lightning_preset_params,
    extra: [
        pack_lightning_ubo,
        lightning_ubo_size,
        lightning_max_waypoints,
        lightning_segments_ubo_size,
    ]
}

#[pyfunction]
#[pyo3(signature = (params, time, position, rotation, view, proj, waypoints=Vec::new()))]
pub fn pack_lightning_ubo(
    py: Python<'_>,
    params: &Bound<'_, PyDict>,
    time: f32,
    position: [f32; 3],
    rotation: [f32; 4],
    view: [f32; 16],
    proj: [f32; 16],
    waypoints: Vec<[f32; 3]>,
) -> PyResult<(Vec<u8>, Vec<u8>, u32)> {
    let mut effect: LightningEffect =
        build_effect_from_params(py, params, time, position, rotation)?;
    let waypoint_count = waypoints.len().min(LIGHTNING_MAX_WAYPOINTS);
    effect.shape.waypoints[..waypoint_count].copy_from_slice(&waypoints[..waypoint_count]);
    effect.shape.waypoint_count = waypoint_count as u32;

    let inv_view_proj = inverse_view_proj_from_column_major(view, proj);
    let (ubo, segments_ubo) = build_lightning_ubo(&effect, inv_view_proj);
    let count = ubo.shape[2] as u32;
    Ok((gpu_block_bytes(&ubo), gpu_block_bytes(&segments_ubo), count))
}

#[pyfunction]
pub fn lightning_ubo_size() -> usize {
    std::mem::size_of::<LightningUBO>()
}

#[pyfunction]
pub fn lightning_max_waypoints() -> usize {
    LIGHTNING_MAX_WAYPOINTS
}

#[pyfunction]
pub fn lightning_segments_ubo_size() -> usize {
    std::mem::size_of::<LightningSegmentsUBO>()
}
