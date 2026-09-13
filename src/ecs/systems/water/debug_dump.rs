use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::ecs::component::{AppliedWaterPreset, WaterTemporalAccum, WaterTorusEffect};
use crate::ecs::resource::{WaterHistorySnapshotState, WaterRenderSettings};
use crate::ecs::systems::effect_debug_dump::{
    build_camera_json, build_light_json, build_projection_json, build_scene_json, matrix_json,
};
use crate::ecs::world::{Entity, Name, World};
use thyllore_effect_core::water::analytic::laplace_beltrami_basis::compute_laplace_beltrami_modes_cached;
use thyllore_effect_core::{build_water_ubo, generate_water_wave_modes, WaterUBO};

pub const WATER_DEBUG_DUMP_DIRECTORY: &str = "log/water";

/// Render-side facts that only the platform layer can observe (GPU, image sizes,
/// acceleration structure contents).
#[derive(Clone, Debug, Default)]
pub struct WaterDebugRenderInfo {
    pub gpu_name: String,
    pub driver_version: String,
    pub api_version: String,
    pub swapchain_size: [u32; 2],
    pub hdr_buffer_size: Option<[u32; 2]>,
    pub water_buffer_size: Option<[u32; 2]>,
    pub mesh_count: usize,
    pub mesh_blas_count: usize,
    pub water_blas_count: usize,
    pub hit_shading_table_capacity: Option<usize>,
    pub screenshot_path: Option<String>,
    pub caustic_accum_path: Option<String>,
    pub caustic_accum_nonzero: Option<u64>,
    pub caustic_accum_max: Option<u32>,
    pub tlas_instances: Option<Value>,
    pub mesh_vertex_probe: Option<Value>,
}

pub fn build_water_debug_record(
    world: &World,
    render_info: &WaterDebugRenderInfo,
    unix_time: u64,
) -> Value {
    let waters: Vec<Value> = world
        .query_waters()
        .into_iter()
        .enumerate()
        .map(|(index, entity)| build_water_instance_json(world, entity, index))
        .collect();

    json!({
        "unix_time": unix_time,
        "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "render": build_render_info_json(render_info),
        "scene": build_scene_json(world),
        "camera": build_camera_json(world),
        "projection": build_projection_json(world),
        "light": build_light_json(world),
        "water_render_settings": build_render_settings_json(world),
        "temporal_state": build_temporal_state_json(world),
        "water_instances": waters,
    })
}

pub fn write_water_debug_dump(record: &Value, unix_time: u64) -> std::io::Result<PathBuf> {
    let directory = Path::new(WATER_DEBUG_DUMP_DIRECTORY);
    std::fs::create_dir_all(directory)?;
    let path = directory.join(format!("water_debug_{unix_time}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(record)?)?;
    Ok(path)
}

pub fn water_debug_screenshot_path(unix_time: u64) -> PathBuf {
    Path::new(WATER_DEBUG_DUMP_DIRECTORY).join(format!("water_debug_{unix_time}.png"))
}

pub fn water_debug_caustic_accum_path(unix_time: u64) -> PathBuf {
    Path::new(WATER_DEBUG_DUMP_DIRECTORY).join(format!("water_debug_{unix_time}_caustic.npy"))
}

fn build_render_info_json(info: &WaterDebugRenderInfo) -> Value {
    json!({
        "gpu_name": info.gpu_name,
        "driver_version": info.driver_version,
        "api_version": info.api_version,
        "swapchain_size": info.swapchain_size,
        "hdr_buffer_size": info.hdr_buffer_size,
        "water_buffer_size": info.water_buffer_size,
        "mesh_count": info.mesh_count,
        "tlas": {
            "mesh_blas_count": info.mesh_blas_count,
            "water_blas_count": info.water_blas_count,
            "hit_shading_table_capacity": info.hit_shading_table_capacity,
        },
        "screenshot_path": info.screenshot_path,
        "caustic_accum_path": info.caustic_accum_path,
        "caustic_accum_nonzero": info.caustic_accum_nonzero,
        "caustic_accum_max": info.caustic_accum_max,
        "tlas_instances": info.tlas_instances,
        "mesh_vertex_probe": info.mesh_vertex_probe,
    })
}

fn build_render_settings_json(world: &World) -> Value {
    let Some(settings) = world.get_resource::<WaterRenderSettings>() else {
        return Value::Null;
    };
    json!({
        "secondary_rays": settings.secondary_rays.label(),
        "secondary_rays_shader_value": settings.secondary_rays.as_shader_value(),
        "debug_view": settings.debug_view,
    })
}

fn build_temporal_state_json(world: &World) -> Value {
    let Some(state) = world.get_resource::<WaterHistorySnapshotState>() else {
        return Value::Null;
    };
    json!({
        "has_previous_snapshot": state.previous.is_some(),
        "previous_view": state.previous.as_ref().map(|s| matrix_json(&s.view)),
    })
}

fn build_water_instance_json(world: &World, entity: Entity, index: usize) -> Value {
    let Some(effect) = world.get_component::<WaterTorusEffect>(entity) else {
        return Value::Null;
    };
    let name = world
        .get_component::<Name>(entity)
        .map(|n| n.0.clone())
        .unwrap_or_default();
    let preset = world
        .get_component::<AppliedWaterPreset>(entity)
        .map(|p| p.name.clone());
    let accum = world
        .get_component::<WaterTemporalAccum>(entity)
        .cloned()
        .unwrap_or_default();
    let ubo = build_water_ubo(effect, accum.frame_index as u32);

    json!({
        "instance_index": index,
        "entity": entity,
        "name": name,
        "applied_preset": preset,
        "effect": build_effect_json(effect),
        "temporal": {
            "weight": accum.weight,
            "frame_index": accum.frame_index,
        },
        "wave_modes": build_wave_modes_json(effect, accum.frame_index as u32),
        "lb_modes": build_laplace_beltrami_modes_json(effect),
        "ubo": build_ubo_json(&ubo),
    })
}

fn build_effect_json(effect: &WaterTorusEffect) -> Value {
    let mut value = serde_json::to_value(effect).unwrap_or(Value::Null);
    if let Some(object) = value.as_object_mut() {
        object.insert("time".to_string(), json!(effect.time));
        object.insert("time_scale".to_string(), json!(effect.time_scale));
        object.insert("time_offset".to_string(), json!(effect.time_offset));
        object.insert("absorption".to_string(), json!(effect.absorption));
        object.insert("tint".to_string(), json!(effect.tint));
    }
    value
}

fn build_wave_modes_json(effect: &WaterTorusEffect, frame_index: u32) -> Value {
    let modes = generate_water_wave_modes(
        effect.wave_amplitude * (1.0 - effect.wave_lb_blend),
        effect.wave_frequency,
        effect.wave_speed,
        effect.wave_dispersion,
        frame_index,
    );
    Value::Array(
        modes
            .iter()
            .map(|mode| {
                json!({
                    "m": mode.m,
                    "n": mode.n,
                    "amplitude": mode.amplitude,
                    "omega": mode.omega,
                    "phase": mode.phase,
                })
            })
            .collect(),
    )
}

fn build_laplace_beltrami_modes_json(effect: &WaterTorusEffect) -> Value {
    if effect.wave_lb_blend <= 0.0 {
        return Value::Array(Vec::new());
    }
    let modes = compute_laplace_beltrami_modes_cached(effect.major_radius, effect.minor_radius);
    Value::Array(
        modes
            .iter()
            .map(|mode| {
                json!({
                    "m": mode.m,
                    "lambda": mode.lambda,
                    "phi_cheb": mode.phi_cheb,
                    "dphi_cheb": mode.dphi_cheb,
                })
            })
            .collect(),
    )
}

fn build_ubo_json(ubo: &WaterUBO) -> Value {
    json!({
        "model": matrix_json(&ubo.model),
        "inverse_model": matrix_json(&ubo.inverse_model),
        "radii": ubo.radii,
        "absorption": ubo.absorption,
        "flow": ubo.flow,
        "composite": ubo.composite,
        "tint": ubo.tint,
        "lighting": ubo.lighting,
        "scattering": ubo.scattering,
        "temporal": ubo.temporal,
        "wave_modes": ubo.wave_modes,
        "lb_modes": ubo.lb_modes,
    })
}
