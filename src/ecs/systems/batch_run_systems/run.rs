use std::path::Path;

use serde_json::json;

use crate::ecs::resource::{BatchDumpPlan, BatchRun, BatchRunState, Camera};
use crate::ecs::world::World;

pub fn batch_run_tick(world: &World) {
    if !world.contains_resource::<BatchRun>() {
        return;
    }

    let mut batch = world.resource_mut::<BatchRun>();
    batch.frames_rendered += 1;
    if matches!(batch.state, BatchRunState::WaitingForFrame)
        && batch.frames_rendered >= batch.screenshot_frame
    {
        batch.state = BatchRunState::ScreenshotRequested;
    }
}

fn format_camera_string(world: &World) -> String {
    if let Some(camera) = world.get_resource::<Camera>() {
        format!(
            "{:.1},{:.1},{:.2}",
            camera.yaw.to_degrees(),
            camera.pitch.to_degrees(),
            camera.distance
        )
    } else {
        "default".to_string()
    }
}

pub fn batch_run_record_screenshot(world: &World, save_result: Result<String, String>) {
    let Some(mut batch) = world.get_resource_mut::<BatchRun>() else {
        return;
    };
    if !matches!(batch.state, BatchRunState::ScreenshotRequested) {
        return;
    }

    let sequence_dir = batch.sequence_dir.clone();
    if let Some(sequence_dir) = sequence_dir {
        let saved = match save_result {
            Ok(path) => path,
            Err(e) => {
                batch.state = BatchRunState::Completed { result: Err(e) };
                return;
            }
        };

        let frame_index = batch.total_count - batch.captures_remaining;
        let frame_path = sequence_dir.join(format!("frame_{:02}.png", frame_index));

        if let Some(parent) = frame_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                batch.state = BatchRunState::Completed {
                    result: Err(format!("failed to create {}: {e}", parent.display())),
                };
                return;
            }
        }

        let copy_result = std::fs::copy(&saved, &frame_path)
            .map_err(|e| format!("failed to copy {} to {}: {e}", saved, frame_path.display()));

        if let Err(e) = std::fs::remove_file(&saved) {
            log_warn!("failed to remove intermediate screenshot {saved}: {e}");
        }

        batch.captures_remaining -= 1;

        if batch.captures_remaining > 0 {
            batch.screenshot_frame += batch.stride as u64;
            batch.state = BatchRunState::WaitingForFrame;
        } else {
            let camera_str = format_camera_string(world);
            let flame_set = world
                .get_resource::<BatchDumpPlan>()
                .map(|plan| plan.flame_set.clone())
                .unwrap_or_default();
            let meta = json!({
                "fps": 60.0 / batch.stride as f64,
                "count": batch.stride,
                "stride": batch.stride,
                "camera": camera_str,
                "flame_set": flame_set.iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
                "engine_args": std::env::args().collect::<Vec<_>>(),
            });

            let meta_path = sequence_dir.join("meta.json");
            if let Err(e) = std::fs::write(
                &meta_path,
                serde_json::to_string_pretty(&meta).unwrap_or_default(),
            ) {
                batch.state = BatchRunState::Completed {
                    result: Err(format!("failed to write {}: {e}", meta_path.display())),
                };
                return;
            }

            match copy_result {
                Ok(_) => {
                    batch.state = BatchRunState::Completed {
                        result: Ok(frame_path.to_string_lossy().to_string()),
                    };
                }
                Err(e) => {
                    batch.state = BatchRunState::Completed { result: Err(e) };
                }
            }
        }
        return;
    }

    let result = save_result.and_then(|saved| move_screenshot_to_output(&saved, &batch.output));

    let (flame_trace_path, wall_probe_path) = world
        .get_resource::<BatchDumpPlan>()
        .map(|plan| (plan.flame_trace_path.clone(), plan.wall_probe_path.clone()))
        .unwrap_or((None, None));
    if flame_trace_path.is_some() || wall_probe_path.is_some() {
        crate::ecs::systems::batch_run_flame_dump(
            world,
            flame_trace_path.as_deref(),
            wall_probe_path.as_deref(),
        );
    }

    batch.state = BatchRunState::Completed { result };
}

fn move_screenshot_to_output(saved: &str, output: &Path) -> Result<String, String> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }
    std::fs::copy(saved, output)
        .map_err(|e| format!("failed to copy {saved} to {}: {e}", output.display()))?;
    if let Err(e) = std::fs::remove_file(saved) {
        log_warn!("failed to remove intermediate screenshot {saved}: {e}");
    }
    Ok(output.to_string_lossy().to_string())
}

pub fn batch_run_is_completed(world: &World) -> bool {
    world
        .get_resource::<BatchRun>()
        .map(|batch| batch.is_completed())
        .unwrap_or(false)
}

pub fn batch_run_report(batch: &BatchRun) -> (bool, String) {
    match &batch.state {
        BatchRunState::Completed { result: Ok(path) } => {
            (true, serde_json::json!({"ok": true, "path": path}).to_string())
        }
        BatchRunState::Completed { result: Err(error) } => (
            false,
            serde_json::json!({"ok": false, "error": error}).to_string(),
        ),
        BatchRunState::WaitingForFrame | BatchRunState::ScreenshotRequested => (
            false,
            serde_json::json!({"ok": false, "error": "batch run ended before screenshot completed"})
                .to_string(),
        ),
    }
}
