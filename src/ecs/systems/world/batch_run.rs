use std::path::{Path, PathBuf};

use serde_json::json;

use crate::ecs::resource::{
    AppExit, BatchRun, BatchRunState, Camera, CaptureOutput, CaptureSchedule, FrameClock,
};
use crate::ecs::world::World;
use crate::hooks::batch_capture::CaptureSlot;

/// Pre-render: advances the frame clock.
pub fn run_frame_clock_phase(world: &World) {
    world.resource_mut::<FrameClock>().advance();
}

/// Pre-render: asks for a capture once the clock reaches the schedule's next frame.
pub fn run_batch_schedule_phase(world: &World) {
    let Some(mut batch) = world.get_resource_mut::<BatchRun>() else {
        return;
    };
    let frame = world.resource::<FrameClock>().frame;

    let BatchRunState::WaitingForFrame {
        next_frame,
        captured,
    } = batch.state
    else {
        return;
    };
    if frame >= next_frame {
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
            request_exit(world);
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
    request_exit(world);
}

fn request_exit(world: &World) {
    if let Some(mut exit) = world.get_resource_mut::<AppExit>() {
        exit.request();
    }
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

    fn batch_world(batch: BatchRun) -> World {
        let mut world = World::new();
        world.insert_resource(FrameClock::fixed(FrameClock::BATCH_DELTA_SECONDS));
        world.insert_resource(AppExit::default());
        world.insert_resource(batch);
        world
    }

    fn run_frame(world: &World) {
        run_frame_clock_phase(world);
        run_batch_schedule_phase(world);
    }

    #[test]
    fn schedule_requests_the_capture_at_its_frame() {
        let world = batch_world(single_run(2));

        run_frame(&world);
        assert!(requested_capture(&world).is_none());

        run_frame(&world);
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
        let mut world = World::new();
        world.insert_resource(FrameClock::wall_clock());
        run_frame(&world);
        assert!(requested_capture(&world).is_none());
        assert_eq!(world.resource::<FrameClock>().frame, 1);
    }

    #[test]
    fn record_completes_a_single_capture_with_its_path() {
        let world = batch_world(single_run(1));
        run_frame(&world);

        batch_run_record_capture(&world, Ok("/tmp/out.png".to_string()));

        let batch = world.resource::<BatchRun>();
        assert_eq!(
            batch.state,
            BatchRunState::Completed {
                result: Ok("/tmp/out.png".to_string())
            }
        );
        assert!(world.resource::<AppExit>().is_requested());
        let (ok, line) = batch_run_report(&batch);
        assert!(ok);
        assert!(line.contains("/tmp/out.png"));
    }

    #[test]
    fn record_ignores_a_result_when_no_capture_was_requested() {
        let world = batch_world(single_run(100));

        batch_run_record_capture(&world, Ok("/tmp/out.png".to_string()));
        assert!(!world.resource::<AppExit>().is_requested());
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
        let world = batch_world(single_run(1));
        run_frame(&world);

        batch_run_record_capture(&world, Err("save failed".to_string()));

        let batch = world.resource::<BatchRun>();
        assert!(batch.is_completed());
        assert!(world.resource::<AppExit>().is_requested());
        let (ok, line) = batch_run_report(&batch);
        assert!(!ok);
        assert!(line.contains("save failed"));
    }

    #[test]
    fn sequence_waits_stride_frames_between_captures_and_writes_meta_at_the_end() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = temp_dir.path().to_path_buf();
        let world = batch_world(BatchRun::new(CaptureSchedule {
            first_frame: 2,
            stride: 3,
            count: 2,
            output: CaptureOutput::Sequence(dir.clone()),
        }));

        for _ in 0..2 {
            run_frame(&world);
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
            run_frame(&world);
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
