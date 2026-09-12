use super::effect::{
    build_effect_from_params, declare_effect_pyfunctions, gpu_block_bytes,
    inverse_view_proj_from_column_major, PyEffect,
};
use crate::water::{
    apply_water_preset, build_water_model_matrix, build_water_ubo,
    overwrite_water_persisted_fields, WaterTorusEffect, WaterUBO, WATER_PRESET_NAMES,
    WATER_UI_PARAMS,
};
use cgmath::{Quaternion, Vector3, Vector4};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use thyllore_math_core::torus_local_bounds_corners;
use thyllore_scene_core::UiParam;

impl PyEffect for WaterTorusEffect {
    const PRESET_NAMES: &'static [&'static str] = WATER_PRESET_NAMES;
    const UI_PARAMS: &'static [UiParam] = WATER_UI_PARAMS;

    fn apply_preset(&mut self, name: &str) -> bool {
        apply_water_preset(self, name)
    }

    fn overwrite_persisted_fields(&mut self, source: &Self) {
        overwrite_water_persisted_fields(self, source);
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
    effect: WaterTorusEffect,
    preset_names: water_preset_names,
    ui_params: water_ui_params,
    preset_params: water_preset_params,
    extra: [pack_water_ubo, water_bounds_corners, water_ubo_size]
}

#[pyfunction]
#[pyo3(signature = (params, time, position, rotation, view, proj, frame_index=0))]
pub fn pack_water_ubo(
    py: Python<'_>,
    params: &Bound<'_, PyDict>,
    time: f32,
    position: [f32; 3],
    rotation: [f32; 4],
    view: [f32; 16],
    proj: [f32; 16],
    frame_index: u32,
) -> PyResult<Vec<u8>> {
    let effect: WaterTorusEffect = build_effect_from_params(py, params, time, position, rotation)?;
    let mut ubo = build_water_ubo(&effect, frame_index);
    ubo.inv_view_proj = inverse_view_proj_from_column_major(view, proj);
    Ok(gpu_block_bytes(&ubo))
}

#[pyfunction]
pub fn water_bounds_corners(
    py: Python<'_>,
    params: &Bound<'_, PyDict>,
    position: [f32; 3],
    rotation: [f32; 4],
) -> PyResult<Vec<[f32; 3]>> {
    let effect: WaterTorusEffect = build_effect_from_params(py, params, 0.0, position, rotation)?;
    let model = build_water_model_matrix(&effect);

    Ok(
        torus_local_bounds_corners(effect.major_radius, effect.minor_radius)
            .iter()
            .map(|corner| {
                let world = model * Vector4::new(corner.x, corner.y, corner.z, 1.0);
                [world.x, world.y, world.z]
            })
            .collect(),
    )
}

#[pyfunction]
pub fn water_ubo_size() -> usize {
    std::mem::size_of::<WaterUBO>()
}
