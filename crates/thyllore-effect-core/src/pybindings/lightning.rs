use super::effect::{
    build_effect_from_params, declare_effect_pyfunctions, gpu_block_bytes,
    inverse_view_proj_from_column_major, PyEffect,
};
use crate::lightning::{
    apply_lightning_preset, build_lightning_ubo, overwrite_lightning_persisted_fields,
    LightningEffect, LightningSegmentsUBO, LightningUBO, LIGHTNING_MAX_WAYPOINTS,
    LIGHTNING_PRESET_NAMES, LIGHTNING_UI_PARAMS,
};
use cgmath::{Quaternion, Vector3};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use thyllore_scene_core::UiParam;

impl PyEffect for LightningEffect {
    const PRESET_NAMES: &'static [&'static str] = LIGHTNING_PRESET_NAMES;
    const UI_PARAMS: &'static [UiParam] = LIGHTNING_UI_PARAMS;

    fn apply_preset(&mut self, name: &str) -> bool {
        apply_lightning_preset(self, name)
    }

    fn overwrite_persisted_fields(&mut self, source: &Self) {
        overwrite_lightning_persisted_fields(self, source);
    }

    fn set_placement(&mut self, time: f32, position: [f32; 3], rotation: [f32; 4]) {
        self.time = time;
        self.position = Vector3::new(position[0], position[1], position[2]);
        self.rotation = Quaternion::new(rotation[0], rotation[1], rotation[2], rotation[3]);
    }

    fn parameter_owner_name(_name: &str) -> &'static str {
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
    effect.waypoints[..waypoint_count].copy_from_slice(&waypoints[..waypoint_count]);
    effect.waypoint_count = waypoint_count as u32;

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
pub fn lightning_segments_ubo_size() -> usize {
    std::mem::size_of::<LightningSegmentsUBO>()
}
