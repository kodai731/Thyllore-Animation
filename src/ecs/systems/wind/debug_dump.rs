use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::ecs::component::{AppliedWindPreset, WindTornadoEffect};
use crate::ecs::resource::WindRenderSettings;
use crate::ecs::systems::effect_debug_dump::{
    build_camera_json, build_light_json, build_projection_json, build_scene_json, matrix_json,
};
use crate::ecs::world::{Entity, Name, World};
use thyllore_effect_core::{build_wind_ubo, wind_parameter_snapshot, WindShadowSlot, WindUBO};

pub const WIND_DEBUG_DUMP_DIRECTORY: &str = "log/wind";

pub fn build_wind_debug_record(
    world: &World,
    screenshot_path: Option<&Path>,
    unix_time: u64,
) -> Value {
    let winds: Vec<Value> = world
        .query_winds()
        .into_iter()
        .enumerate()
        .map(|(index, entity)| build_wind_instance_json(world, entity, index))
        .collect();

    json!({
        "unix_time": unix_time,
        "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "screenshot_path": screenshot_path.map(|p| p.display().to_string()),
        "scene": build_scene_json(world),
        "camera": build_camera_json(world),
        "projection": build_projection_json(world),
        "light": build_light_json(world),
        "wind_render_settings": build_render_settings_json(world),
        "wind_instances": winds,
    })
}

pub fn write_wind_debug_dump(record: &Value, unix_time: u64) -> std::io::Result<PathBuf> {
    let directory = Path::new(WIND_DEBUG_DUMP_DIRECTORY);
    std::fs::create_dir_all(directory)?;
    let path = directory.join(format!("wind_debug_{unix_time}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(record)?)?;
    Ok(path)
}

pub fn wind_debug_screenshot_path(unix_time: u64) -> PathBuf {
    Path::new(WIND_DEBUG_DUMP_DIRECTORY).join(format!("wind_debug_{unix_time}.png"))
}

fn build_render_settings_json(world: &World) -> Value {
    let Some(settings) = world.get_resource::<WindRenderSettings>() else {
        return Value::Null;
    };
    json!({
        "shading_mode": settings.shading_mode.label(),
        "reference_step_count": settings.reference_step_count,
        "debug_view": settings.debug_view.label(),
        "batch_fixed_time": settings.batch_fixed_time,
        "free_run_when_paused": settings.free_run_when_paused,
        "resolve_scale": format!("{:?}", settings.resolve_scale),
    })
}

fn build_wind_instance_json(world: &World, entity: Entity, index: usize) -> Value {
    let Some(effect) = world.get_component::<WindTornadoEffect>(entity) else {
        return Value::Null;
    };
    let name = world
        .get_component::<Name>(entity)
        .map(|n| n.0.clone())
        .unwrap_or_default();
    let preset = world
        .get_component::<AppliedWindPreset>(entity)
        .map(|p| p.name.clone());
    let ubo = build_wind_ubo(&effect, WindShadowSlot(index as u32));

    json!({
        "instance_index": index,
        "entity": entity,
        "name": name,
        "applied_preset": preset,
        "effect": build_effect_json(&effect),
        "ubo": build_ubo_json(&ubo),
    })
}

fn build_effect_json(effect: &WindTornadoEffect) -> Value {
    let mut fields = serde_json::Map::new();
    for (key, values) in wind_parameter_snapshot(effect) {
        let value = match values.as_slice() {
            [single] => json!(single),
            many => json!(many),
        };
        fields.insert(key.to_string(), value);
    }
    fields.insert("time".to_string(), json!(effect.time));
    fields.insert("time_scale".to_string(), json!(effect.time_scale));
    fields.insert("time_offset".to_string(), json!(effect.time_offset));
    Value::Object(fields)
}

fn build_ubo_json(ubo: &WindUBO) -> Value {
    let puffs: Vec<[f32; 4]> = ubo.puffs[..ubo.puff_params[0] as usize].to_vec();
    json!({
        "model": matrix_json(&ubo.model),
        "inverse_model": matrix_json(&ubo.inverse_model),
        "shape": ubo.shape,
        "wall": ubo.wall,
        "optics": ubo.optics,
        "albedo": ubo.albedo,
        "lighting": ubo.lighting,
        "streak": ubo.streak,
        "streak2": ubo.streak2,
        "eddy": ubo.eddy,
        "eddy2": ubo.eddy2,
        "puff_params": ubo.puff_params,
        "puffs": puffs,
    })
}
