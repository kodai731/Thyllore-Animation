use std::path::{Path, PathBuf};

use serde_json::json;

use crate::ecs::resource::{BatchRun, BatchRunState, Camera, CaptureOutput, CaptureSchedule};
use crate::ecs::world::World;
use crate::hooks::batch_capture::CaptureSlot;

/// Pre-render: counts the frame and asks for a capture once the schedule's next frame is reached.
pub fn run_batch_schedule_phase(world: &World) {
    let Some(mut batch) = world.get_resource_mut::<BatchRun>() else {
        return;
    };
    batch.frames_rendered += 1;

    let BatchRunState::WaitingForFrame {
        next_frame,
        captured,
    } = batch.state
    else {
        return;
    };
    if batch.frames_rendered >= next_frame {
        batch.state = BatchRunState::CaptureRequested { index: captured };
    }
}

/// The capture the schedule is waiting on this frame, if any.
pub fn requested_capture(world: &World) -> Option<CaptureSlot> {
    let batch = world.get_resource::<BatchRun>()?;
    let BatchRunState::CaptureRequested { index } = batch.state else {
        return None;
    };
    Some(CaptureSlot {
        index,
        count: batch.capture.count,
        sequence_dir: batch.capture.output.sequence_dir().map(Path::to_path_buf),
    })
}

pub fn capture_output_path(schedule: &CaptureSchedule, index: u32) -> PathBuf {
    match &schedule.output {
        CaptureOutput::Single(path) => path.clone(),
        CaptureOutput::Sequence(dir) => dir.join(format!("frame_{index:02}.png")),
    }
}

/// Post-render: stores the capture result and either waits for the next frame or completes.
pub fn batch_run_record_capture(world: &World, saved: Result<String, String>) {
    let Some(mut batch) = world.get_resource_mut::<BatchRun>() else {
        return;
    };
    let BatchRunState::CaptureRequested { index } = batch.state else {
        return;
    };

    let saved_path = match saved {
        Ok(path) => path,
        Err(error) => {
            batch.state = BatchRunState::Completed { result: Err(error) };
            return;
        }
    };

    let captured = index + 1;
    if captured < batch.capture.count {
        batch.state = BatchRunState::WaitingForFrame {
            next_frame: batch.capture.frame_of_capture(captured),
            captured,
        };
        return;
    }

    let result = match &batch.capture.output {
        CaptureOutput::Single(_) => Ok(saved_path),
        CaptureOutput::Sequence(dir) => {
            write_sequence_meta(dir, &batch.capture, world).map(|_| saved_path)
        }
    };
    batch.state = BatchRunState::Completed { result };
}

fn write_sequence_meta(
    dir: &Path,
    schedule: &CaptureSchedule,
    world: &World,
) -> Result<(), String> {
    let meta = json!({
        "fps": 60.0 / schedule.stride as f64,
        "count": schedule.count,
        "stride": schedule.stride,
        "camera": format_camera_string(world),
        "engine_args": std::env::args().collect::<Vec<_>>(),
    });
    let meta_path = dir.join("meta.json");
    std::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    )
    .map_err(|e| format!("failed to write {}: {e}", meta_path.display()))
}

fn format_camera_string(world: &World) -> String {
    match world.get_resource::<Camera>() {
        Some(camera) => format!(
            "{:.1},{:.1},{:.2}",
            camera.yaw.to_degrees(),
            camera.pitch.to_degrees(),
            camera.distance
        ),
        None => "default".to_string(),
    }
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
            (true, json!({"ok": true, "path": path}).to_string())
        }
        BatchRunState::Completed { result: Err(error) } => {
            (false, json!({"ok": false, "error": error}).to_string())
        }
        BatchRunState::WaitingForFrame { .. } | BatchRunState::CaptureRequested { .. } => (
            false,
            json!({"ok": false, "error": "batch run ended before screenshot completed"})
                .to_string(),
        ),
    }
}
