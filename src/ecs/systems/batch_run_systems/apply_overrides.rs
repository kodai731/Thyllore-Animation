use crate::asset::AssetStorage;
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{
    BatchDumpPlan, BatchPickRequest, BatchRun, Camera, ExposureDumpSink, GpuTimingsSink,
    ModelState, TimelineState,
};
use crate::ecs::systems::clip_library_systems::find_best_clip;
use crate::ecs::world::World;

use super::anim_edits::batch_apply_anim_edits;
use super::batch_action::BatchAction;
use super::cli_resolve::{BatchCameraPose, EngineCliOverrides};
use super::debug_actions::batch_apply_debug_actions;

/// A batch run also gets an empty `BatchDumpPlan` that the dump actions and the hooks fill in.
pub fn apply_engine_overrides(
    world: &mut World,
    assets: &mut AssetStorage,
    overrides: &EngineCliOverrides,
) {
    if let Some(batch_run) = &overrides.batch_run {
        world.insert_resource(batch_run.clone());
        world.insert_resource(BatchDumpPlan::default());
    }
    if let Some(pose) = overrides.camera_pose {
        apply_camera_pose(world, pose);
    }
    if let Some(path) = overrides.gpu_timings_path.clone() {
        world.insert_resource(GpuTimingsSink::new(path));
    }
    if let Some(path) = overrides.exposure_dump_path.clone() {
        world.insert_resource(ExposureDumpSink::new(path));
    }
    if overrides.scene_path.is_some() {
        request_scene_model_load(world);
    }
    if let Some(pixel) = overrides.pick_pixel {
        world.insert_resource(BatchPickRequest::new(pixel));
    }
    if !overrides.anim_edits.is_empty() {
        batch_apply_anim_edits(world, assets, &overrides.anim_edits);
    }

    let actions: Vec<&dyn BatchAction> = overrides.debug_actions.iter().map(Box::as_ref).collect();
    batch_apply_debug_actions(world, &actions);

    if overrides.batch_play {
        start_batch_playback(world);
    }
}

fn apply_camera_pose(world: &mut World, pose: BatchCameraPose) {
    let mut camera = world.resource_mut::<Camera>();
    camera.yaw = pose.yaw_degrees.to_radians();
    camera.pitch = pose.pitch_degrees.to_radians();
    camera.distance = pose.distance;
    if let Some(pivot) = pose.pivot {
        camera.pivot = cgmath::Vector3::new(pivot[0], pivot[1], pivot[2]);
    }
}

/// A scene file restores its model path into `ModelState`; loading it reuses the UI's event.
fn request_scene_model_load(world: &mut World) {
    let model_path = world.resource::<ModelState>().model_path.clone();
    if model_path.is_empty() || model_path == "Generated Mesh" {
        return;
    }
    world
        .resource_mut::<UIEventQueue>()
        .send(UIEvent::LoadModel { path: model_path });
}

/// Prefers a clip with bone tracks so an empty default clip never shadows the model animation.
fn start_batch_playback(world: &mut World) {
    let first = find_best_clip(world);
    let mut timeline = world.resource_mut::<TimelineState>();
    timeline.playing = true;
    timeline.looping = true;
    timeline.current_time = 0.0;
    if timeline.current_clip_id.is_none() {
        timeline.current_clip_id = first;
    }
    drop(timeline);

    if let Some(mut batch_run) = world.get_resource_mut::<BatchRun>() {
        batch_run.play_requested = true;
        batch_run.play_clip_id = first;
    }
}
