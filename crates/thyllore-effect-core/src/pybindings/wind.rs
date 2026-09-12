use super::effect::{
    build_effect_from_params, declare_effect_pyfunctions, gpu_block_bytes,
    inverse_view_proj_from_column_major, PyEffect,
};
use crate::wind::{
    apply_wind_preset, build_wind_model_matrix, build_wind_ubo, overwrite_wind_persisted_fields,
    wind_local_bounds_corners, WindRenderSettings, WindShadowSlot, WindShellParams,
    WindTornadoEffect, WindUBO, WIND_DEFAULT_PRESET, WIND_PRESET_NAMES, WIND_UI_PARAMS,
};
use cgmath::{Quaternion, Vector3, Vector4};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use thyllore_scene_core::UiParam;

impl PyEffect for WindTornadoEffect {
    const PRESET_NAMES: &'static [&'static str] = WIND_PRESET_NAMES;
    const UI_PARAMS: &'static [UiParam] = WIND_UI_PARAMS;

    fn apply_preset(&mut self, name: &str) -> bool {
        apply_wind_preset(self, name)
    }

    fn overwrite_persisted_fields(&mut self, source: &Self) {
        overwrite_wind_persisted_fields(self, source);
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
    effect: WindTornadoEffect,
    preset_names: wind_preset_names,
    ui_params: wind_ui_params,
    preset_params: wind_preset_params,
    extra: [
        wind_default_preset,
        pack_wind_ubo,
        wind_bounds_corners,
        wind_ubo_size,
        wind_resolve_divisor,
    ]
}

#[pyfunction]
pub fn wind_default_preset() -> &'static str {
    WIND_DEFAULT_PRESET
}

#[pyfunction]
#[pyo3(signature = (params, time, position, rotation, view, proj))]
pub fn pack_wind_ubo(
    py: Python<'_>,
    params: &Bound<'_, PyDict>,
    time: f32,
    position: [f32; 3],
    rotation: [f32; 4],
    view: [f32; 16],
    proj: [f32; 16],
) -> PyResult<Vec<u8>> {
    let effect: WindTornadoEffect = build_effect_from_params(py, params, time, position, rotation)?;
    let mut ubo = build_wind_ubo(&effect, WindShadowSlot(0));
    ubo.inv_view_proj = inverse_view_proj_from_column_major(view, proj);
    Ok(gpu_block_bytes(&ubo))
}

/// World-space corners of the envelope box the engine picks and scissors the wind pass against.
#[pyfunction]
#[pyo3(signature = (params, time, position, rotation))]
pub fn wind_bounds_corners(
    py: Python<'_>,
    params: &Bound<'_, PyDict>,
    time: f32,
    position: [f32; 3],
    rotation: [f32; 4],
) -> PyResult<Vec<[f32; 3]>> {
    let effect: WindTornadoEffect = build_effect_from_params(py, params, time, position, rotation)?;

    let model = build_wind_model_matrix(&effect);
    let shell_params = WindShellParams::from_effect(&effect);

    Ok(wind_local_bounds_corners(&shell_params)
        .iter()
        .map(|corner| {
            let world = model * Vector4::new(corner.x, corner.y, corner.z, 1.0);
            [world.x, world.y, world.z]
        })
        .collect())
}

#[pyfunction]
pub fn wind_ubo_size() -> usize {
    std::mem::size_of::<WindUBO>()
}

/// Resolution divisor of the engine's default wind resolve scale (1 = full, 2 = half).
#[pyfunction]
pub fn wind_resolve_divisor() -> u32 {
    WindRenderSettings::default().resolve_scale.divisor()
}
