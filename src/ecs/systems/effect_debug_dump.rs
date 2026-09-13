use cgmath::Matrix4;
use serde_json::{json, Value};

use crate::ecs::resource::{
    Camera, DebugViewState, LightState, ModelState, ProjectionData, SceneState, TimelineState,
};
use crate::ecs::systems::camera_systems::{
    compute_camera_direction, compute_camera_position, compute_camera_right, compute_camera_up,
};
use crate::ecs::world::World;
use thyllore_effect_core::inverse_view_proj_f64;

pub fn build_scene_json(world: &World) -> Value {
    let scene_path = world
        .get_resource::<SceneState>()
        .and_then(|s| s.current_scene_path.clone())
        .map(|p| p.to_string_lossy().to_string());
    let model = world.get_resource::<ModelState>().map(|m| {
        json!({
            "path": m.model_path,
            "load_status": m.load_status,
            "has_skinned_meshes": m.has_skinned_meshes,
        })
    });
    let timeline = world.get_resource::<TimelineState>().map(|t| {
        json!({
            "current_time": t.current_time,
            "playing": t.playing,
            "looping": t.looping,
            "speed": t.speed,
            "current_clip_id": t.current_clip_id.map(|id| format!("{id:?}")),
        })
    });
    let debug_view = world.get_resource::<DebugViewState>().map(|d| {
        json!({
            "view_mode": format!("{:?}", d.debug_view_mode),
            "black_background": d.black_background,
        })
    });

    json!({
        "scene_path": scene_path,
        "model": model,
        "flame_count": world.query_flames().len(),
        "timeline": timeline,
        "debug_view": debug_view,
    })
}

pub fn build_camera_json(world: &World) -> Value {
    let Some(camera) = world.get_resource::<Camera>() else {
        return Value::Null;
    };
    let position = compute_camera_position(&camera);
    let forward = compute_camera_direction(&camera);
    let right = compute_camera_right(&camera);
    let up = compute_camera_up(&camera);
    let batch_camera = format!(
        "{:.3},{:.3},{:.4},{:.4},{:.4},{:.4}",
        camera.yaw.to_degrees(),
        camera.pitch.to_degrees(),
        camera.distance,
        camera.pivot.x,
        camera.pivot.y,
        camera.pivot.z
    );

    json!({
        "pivot": [camera.pivot.x, camera.pivot.y, camera.pivot.z],
        "yaw_degrees": camera.yaw.to_degrees(),
        "pitch_degrees": camera.pitch.to_degrees(),
        "distance": camera.distance,
        "fov_y_degrees": camera.fov_y.0,
        "near_plane": camera.near_plane,
        "position": [position.x, position.y, position.z],
        "forward": [forward.x, forward.y, forward.z],
        "right": [right.x, right.y, right.z],
        "up": [up.x, up.y, up.z],
        "batch_camera": batch_camera,
    })
}

pub fn build_projection_json(world: &World) -> Value {
    let Some(projection) = world.get_resource::<ProjectionData>() else {
        return Value::Null;
    };
    json!({
        "view": matrix_json(&projection.view),
        "proj": matrix_json(&projection.proj),
        "inv_view_proj_f64": matrix_json(&inverse_view_proj_f64(projection.proj, projection.view)),
        "screen_size": [projection.screen_size.x, projection.screen_size.y],
        "aspect": projection.aspect,
    })
}

pub fn build_light_json(world: &World) -> Value {
    let Some(light) = world.get_resource::<LightState>() else {
        return Value::Null;
    };
    json!({
        "position": [light.light_position.x, light.light_position.y, light.light_position.z],
        "shadow_strength": light.shadow_strength,
        "shadow_normal_offset": light.shadow_normal_offset,
    })
}

pub fn matrix_json(matrix: &Matrix4<f32>) -> Value {
    let columns: [[f32; 4]; 4] = (*matrix).into();
    json!(columns)
}

pub fn current_unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
