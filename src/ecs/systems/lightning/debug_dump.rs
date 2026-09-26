use std::path::{Path, PathBuf};

use cgmath::{Matrix4, SquareMatrix};
use serde_json::{json, Value};

use crate::ecs::component::LightningEffect;
use crate::ecs::resource::{LightningRenderSettings, ProjectionData};
use crate::ecs::systems::effect_debug_dump::{
    build_camera_json, build_light_json, build_projection_json, build_scene_json, matrix_json,
};
use crate::ecs::world::{Entity, Name, World};
use thyllore_effect_core::{
    build_lightning_ubo, inverse_view_proj_f64, lightning_parameter_snapshot, LightningSegmentsUBO,
    LightningUBO,
};

pub const LIGHTNING_DEBUG_DUMP_DIRECTORY: &str = "log/lightning";

pub fn build_lightning_debug_record(
    world: &World,
    screenshot_path: Option<&Path>,
    unix_time: u64,
) -> Value {
    let inv_view_proj = world
        .get_resource::<ProjectionData>()
        .map(|projection| inverse_view_proj_f64(projection.proj, projection.view))
        .unwrap_or_else(Matrix4::identity);

    let lightnings: Vec<Value> = world
        .entities_with::<LightningEffect>()
        .into_iter()
        .enumerate()
        .map(|(index, entity)| build_lightning_instance_json(world, entity, index, inv_view_proj))
        .collect();

    json!({
        "unix_time": unix_time,
        "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "screenshot_path": screenshot_path.map(|p| p.display().to_string()),
        "scene": build_scene_json(world),
        "camera": build_camera_json(world),
        "projection": build_projection_json(world),
        "light": build_light_json(world),
        "lightning_render_settings": build_render_settings_json(world),
        "lightning_instances": lightnings,
    })
}

pub fn write_lightning_debug_dump(record: &Value, unix_time: u64) -> std::io::Result<PathBuf> {
    let directory = Path::new(LIGHTNING_DEBUG_DUMP_DIRECTORY);
    std::fs::create_dir_all(directory)?;
    let path = directory.join(format!("lightning_debug_{unix_time}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(record)?)?;
    Ok(path)
}

pub fn lightning_debug_screenshot_path(unix_time: u64) -> PathBuf {
    Path::new(LIGHTNING_DEBUG_DUMP_DIRECTORY).join(format!("lightning_debug_{unix_time}.png"))
}

fn build_render_settings_json(world: &World) -> Value {
    let Some(settings) = world.get_resource::<LightningRenderSettings>() else {
        return Value::Null;
    };
    json!({
        "shading_mode": settings.shading_mode.label(),
        "reference_step_count": settings.reference_step_count,
        "debug_view": settings.debug_view.label(),
        "batch_fixed_time": settings.batch_fixed_time,
    })
}

fn build_lightning_instance_json(
    world: &World,
    entity: Entity,
    index: usize,
    inv_view_proj: Matrix4<f32>,
) -> Value {
    let Some(effect) = world.get_component::<LightningEffect>(entity) else {
        return Value::Null;
    };
    let name = world
        .get_component::<Name>(entity)
        .map(|n| n.0.clone())
        .unwrap_or_default();
    let (ubo, segments) = build_lightning_ubo(&effect, inv_view_proj);

    json!({
        "instance_index": index,
        "entity": entity,
        "name": name,
        "effect": build_effect_json(&effect),
        "ubo": build_ubo_json(&ubo),
        "segments": build_segments_json(&segments),
    })
}

fn build_effect_json(effect: &LightningEffect) -> Value {
    let mut fields = serde_json::Map::new();
    for (key, values) in lightning_parameter_snapshot(effect) {
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

fn build_ubo_json(ubo: &LightningUBO) -> Value {
    json!({
        "model": matrix_json(&ubo.model),
        "inverse_model": matrix_json(&ubo.inverse_model),
        "core": ubo.core,
        "rim": ubo.rim,
        "flash": ubo.flash,
        "shape": ubo.shape,
        "inv_view_proj": matrix_json(&ubo.inv_view_proj),
    })
}

fn build_segments_json(segments: &LightningSegmentsUBO) -> Value {
    let active: Vec<Value> = segments
        .seg_misc
        .iter()
        .enumerate()
        .take_while(|(_, misc)| misc[1] > 0.0)
        .map(|(index, misc)| {
            json!({
                "a_r0": segments.seg_a_r0[index],
                "b_r1": segments.seg_b_r1[index],
                "misc": misc,
            })
        })
        .collect();
    json!(active)
}
