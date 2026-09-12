use super::effect::{
    build_effect_from_params, declare_effect_pyfunctions, gpu_block_bytes, PyEffect,
};
use crate::flame::{
    apply_flame_preset, build_flame_model_matrix, build_flame_ubo, effective_sigma_t,
    flame_bend_offset, flame_local_bounds, flame_local_bounds_corners, flame_proxy_pad,
    flame_support_scale, overwrite_persisted_fields, parameter_owner, refresh_flame_coefficients,
    FlameBaked, FlameEffect, FlameTemporalAccum, FlameUBO, ParameterOwner, FLAME_PRESET_NAMES,
    FLAME_UI_PARAMS, MIN_FLAME_EXTENT,
};
use cgmath::{Quaternion, Vector3, Vector4};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use thyllore_scene_core::UiParam;

impl PyEffect for FlameEffect {
    const PRESET_NAMES: &'static [&'static str] = FLAME_PRESET_NAMES;
    const UI_PARAMS: &'static [UiParam] = FLAME_UI_PARAMS;

    fn apply_preset(&mut self, name: &str) -> bool {
        apply_flame_preset(self, name)
    }

    fn overwrite_persisted_fields(&mut self, source: &Self) {
        overwrite_persisted_fields(self, source);
    }

    fn set_placement(&mut self, time: f32, position: [f32; 3], rotation: [f32; 4]) {
        self.time = time;
        self.position = Vector3::new(position[0], position[1], position[2]);
        self.rotation = Quaternion::new(rotation[0], rotation[1], rotation[2], rotation[3]);
    }

    fn parameter_owner_name(name: &str) -> &'static str {
        match parameter_owner(name) {
            Some(ParameterOwner::Frame) => "frame",
            Some(ParameterOwner::Shape) => "shape",
            Some(ParameterOwner::Style) => "style",
            None => "unknown",
        }
    }
}

declare_effect_pyfunctions! {
    effect: FlameEffect,
    preset_names: flame_preset_names,
    ui_params: flame_ui_params,
    preset_params: flame_preset_params,
    extra: [
        flame_effective_optical_depth,
        pack_flame_ubo,
        flame_ubo_size,
        flame_shader_specialization,
        flame_bounds_corners,
    ]
}

fn build_flame_from_params(
    py: Python<'_>,
    params: &Bound<'_, PyDict>,
    time: f32,
    position: [f32; 3],
    rotation: [f32; 4],
    light_position: Option<[f32; 3]>,
) -> PyResult<(FlameEffect, FlameBaked)> {
    let mut effect: FlameEffect = build_effect_from_params(py, params, time, position, rotation)?;
    if let Some(lp) = light_position {
        effect.light_position_world = Vector3::new(lp[0], lp[1], lp[2]);
    }

    let baked = FlameBaked::default();
    refresh_flame_coefficients(&mut effect, &baked);
    Ok((effect, baked))
}

fn build_ubo_from_params(
    py: Python<'_>,
    params: &Bound<'_, PyDict>,
    time: f32,
    position: [f32; 3],
    rotation: [f32; 4],
    light_position: Option<[f32; 3]>,
    frame_index: u64,
) -> PyResult<FlameUBO> {
    let (effect, baked) =
        build_flame_from_params(py, params, time, position, rotation, light_position)?;

    let temporal = FlameTemporalAccum {
        frame_index,
        ..Default::default()
    };
    Ok(build_flame_ubo(&effect, &baked, &temporal))
}

/// World-space corners of the shell proxy box, the same box the engine scissors its
/// flame pass to (`compute_flame_scissor`) and picks against.
#[pyfunction]
pub fn flame_bounds_corners(
    py: Python<'_>,
    params: &Bound<'_, PyDict>,
    position: [f32; 3],
    rotation: [f32; 4],
) -> PyResult<Vec<[f32; 3]>> {
    let (effect, baked) = build_flame_from_params(py, params, 0.0, position, rotation, None)?;

    let bounds = flame_local_bounds(
        flame_bend_offset(&effect),
        flame_support_scale(&effect),
        effect.support_margin,
        flame_proxy_pad(&effect, &baked),
    );
    let model = build_flame_model_matrix(&effect);

    Ok(flame_local_bounds_corners(&bounds)
        .iter()
        .map(|corner| {
            let world = model * Vector4::new(corner.x, corner.y, corner.z, 1.0);
            [world.x, world.y, world.z]
        })
        .collect())
}

#[pyfunction]
#[pyo3(signature = (params, time, position, rotation, light_position=None, frame_index=0))]
pub fn pack_flame_ubo(
    py: Python<'_>,
    params: &Bound<'_, PyDict>,
    time: f32,
    position: [f32; 3],
    rotation: [f32; 4],
    light_position: Option<[f32; 3]>,
    frame_index: u64,
) -> PyResult<Vec<u8>> {
    let ubo = build_ubo_from_params(
        py,
        params,
        time,
        position,
        rotation,
        light_position,
        frame_index,
    )?;
    Ok(gpu_block_bytes(&ubo))
}

/// Uniform members that only select a code path in `resolveFragment.frag`.
/// The Blender addon bakes them into the GLSL as constants so the dead paths are
/// not compiled; the values come from the same UBO the shader would read.
#[pyfunction]
pub fn flame_shader_specialization<'py>(
    py: Python<'py>,
    params: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyDict>> {
    let ubo = build_ubo_from_params(py, params, 0.0, [0.0; 3], [1.0, 0.0, 0.0, 0.0], None, 0)?;

    let dict = PyDict::new(py);
    dict.set_item("flame.emitterParams.kind", ubo.emitter_params.kind)?;
    dict.set_item("flame.contourParams.rteBands", ubo.contour_params.rte_bands)?;
    dict.set_item("flame.trailMeta.sampleCount", ubo.trail_meta.sample_count)?;
    Ok(dict)
}

/// Optical depth the effect really renders with: `optical_depth` when set,
/// otherwise `sigma_t * radius` (the `0 = use sigma_t directly` convention).
#[pyfunction]
pub fn flame_effective_optical_depth(py: Python<'_>, params: &Bound<'_, PyDict>) -> PyResult<f32> {
    let (effect, _) =
        build_flame_from_params(py, params, 0.0, [0.0; 3], [1.0, 0.0, 0.0, 0.0], None)?;
    Ok(effective_sigma_t(&effect) * effect.radius.max(MIN_FLAME_EXTENT))
}

#[pyfunction]
pub fn flame_ubo_size() -> usize {
    std::mem::size_of::<FlameUBO>()
}
