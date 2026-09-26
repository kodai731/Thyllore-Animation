use super::effect::{
    build_effect_from_params, declare_effect_pyfunctions, gpu_block_bytes,
    inverse_view_proj_from_column_major, ParameterOwnerName,
};
use crate::wind::{
    build_wind_model_matrix, build_wind_ubo, wind_local_bounds_corners, WindRenderSettings,
    WindShadowSlot, WindTornadoEffect, WindUBO, WIND_DEFAULT_PRESET,
};
use crate::WindParameterOwner;
use cgmath::Vector4;
use pyo3::prelude::*;
use pyo3::types::PyDict;

impl ParameterOwnerName for WindParameterOwner {
    fn owner_name(self) -> &'static str {
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
    let ubo = build_wind_ubo(&effect, WindShadowSlot(0));

    Ok(wind_local_bounds_corners(&ubo)
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
