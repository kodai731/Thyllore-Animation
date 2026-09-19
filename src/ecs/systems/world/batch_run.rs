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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::resource::BatchRunState;

    fn single_run(first_frame: u64) -> BatchRun {
        BatchRun::new(CaptureSchedule::single(
            PathBuf::from("/tmp/out.png"),
            first_frame,
        ))
    }

    #[test]
    fn schedule_requests_the_capture_at_its_frame() {
        let mut world = World::new();
        world.insert_resource(single_run(2));

        run_batch_schedule_phase(&world);
        assert!(requested_capture(&world).is_none());

        run_batch_schedule_phase(&world);
        assert_eq!(
            requested_capture(&world),
            Some(CaptureSlot {
                index: 0,
                count: 1,
                sequence_dir: None
            })
        );
    }

    #[test]
    fn schedule_without_a_batch_run_requests_nothing() {
        let world = World::new();
        run_batch_schedule_phase(&world);
        assert!(requested_capture(&world).is_none());
    }

    #[test]
    fn record_completes_a_single_capture_with_its_path() {
        let mut world = World::new();
        world.insert_resource(single_run(1));
        run_batch_schedule_phase(&world);

        batch_run_record_capture(&world, Ok("/tmp/out.png".to_string()));

        let batch = world.resource::<BatchRun>();
        assert_eq!(
            batch.state,
            BatchRunState::Completed {
                result: Ok("/tmp/out.png".to_string())
            }
        );
        let (ok, line) = batch_run_report(&batch);
        assert!(ok);
        assert!(line.contains("/tmp/out.png"));
    }

    #[test]
    fn record_ignores_a_result_when_no_capture_was_requested() {
        let mut world = World::new();
        world.insert_resource(single_run(100));

        batch_run_record_capture(&world, Ok("/tmp/out.png".to_string()));
        assert_eq!(
            world.resource::<BatchRun>().state,
            BatchRunState::WaitingForFrame {
                next_frame: 100,
                captured: 0
            }
        );
    }

    #[test]
    fn record_stores_error_result() {
        let mut world = World::new();
        world.insert_resource(single_run(1));
        run_batch_schedule_phase(&world);

        batch_run_record_capture(&world, Err("save failed".to_string()));

        let batch = world.resource::<BatchRun>();
        assert!(batch.is_completed());
        let (ok, line) = batch_run_report(&batch);
        assert!(!ok);
        assert!(line.contains("save failed"));
    }

    #[test]
    fn sequence_waits_stride_frames_between_captures_and_writes_meta_at_the_end() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = temp_dir.path().to_path_buf();
        let mut world = World::new();
        world.insert_resource(BatchRun::new(CaptureSchedule {
            first_frame: 2,
            stride: 3,
            count: 2,
            output: CaptureOutput::Sequence(dir.clone()),
        }));

        for _ in 0..2 {
            run_batch_schedule_phase(&world);
        }
        let first = requested_capture(&world).expect("first capture at frame 2");
        assert_eq!(first.index, 0);
        assert_eq!(first.sequence_dir.as_deref(), Some(dir.as_path()));
        assert_eq!(
            capture_output_path(&world.resource::<BatchRun>().capture, first.index),
            dir.join("frame_00.png")
        );
        batch_run_record_capture(&world, Ok(dir.join("frame_00.png").display().to_string()));
        assert_eq!(
            world.resource::<BatchRun>().state,
            BatchRunState::WaitingForFrame {
                next_frame: 5,
                captured: 1
            }
        );

        for _ in 0..3 {
            run_batch_schedule_phase(&world);
        }
        let second = requested_capture(&world).expect("second capture at frame 5");
        assert_eq!(second.index, 1);
        batch_run_record_capture(&world, Ok(dir.join("frame_01.png").display().to_string()));

        assert!(world.resource::<BatchRun>().is_completed());
        let meta: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("meta.json")).unwrap()).unwrap();
        assert_eq!(meta["count"], 2);
        assert_eq!(meta["stride"], 3);
        assert!((meta["fps"].as_f64().unwrap() - 20.0).abs() < 1e-9);
    }

    #[test]
    fn report_incomplete_state_is_error() {
        let batch = single_run(1);
        let (ok, line) = batch_run_report(&batch);
        assert!(!ok);
        assert!(line.contains("before screenshot completed"));
    }
}
