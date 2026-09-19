use std::path::{Path, PathBuf};

use super::*;
use crate::asset::AssetStorage;
use crate::ecs::component::{FlameEffect, MotionPath};
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{
    BatchFlameOrbit, BatchRun, BatchRunState, CaptureOutput, CaptureSchedule, ClipLibrary,
    DebugViewMode, DebugViewState, FlameWallProbeCapture, TimelineState,
};
use crate::ecs::world::{Transform, World};

fn single_run(first_frame: u64) -> BatchRun {
    BatchRun::new(CaptureSchedule::single(
        PathBuf::from("/tmp/out.png"),
        first_frame,
    ))
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn pick_pixel_is_absent_without_the_flag() {
    assert_eq!(pick_pixel_resolve_from_args(&args(&["bin"])).unwrap(), None);
}

#[test]
fn pick_pixel_parses_a_pixel_pair() {
    assert_eq!(
        pick_pixel_resolve_from_args(&args(&["bin", "--batch-pick", "947,150"])).unwrap(),
        Some((947, 150))
    );
}

#[test]
fn pick_pixel_rejects_a_malformed_pair() {
    assert!(pick_pixel_resolve_from_args(&args(&["bin", "--batch-pick", "947"])).is_err());
    assert!(pick_pixel_resolve_from_args(&args(&["bin", "--batch-pick", "a,b"])).is_err());
}

#[test]
fn resolve_returns_none_without_flag() {
    let resolved = batch_run_resolve_from_args(&args(&["thyllore-animation"])).unwrap();
    assert!(resolved.is_none());
}

#[test]
fn resolve_parses_output_and_default_frames() {
    let resolved =
        batch_run_resolve_from_args(&args(&["bin", "--batch-screenshot", "/tmp/out.png"]))
            .unwrap()
            .unwrap();
    assert_eq!(
        resolved.capture,
        CaptureSchedule::single(PathBuf::from("/tmp/out.png"), DEFAULT_SCREENSHOT_FRAME)
    );
    assert_eq!(
        resolved.state,
        BatchRunState::WaitingForFrame {
            next_frame: DEFAULT_SCREENSHOT_FRAME,
            captured: 0
        }
    );
}

#[test]
fn resolve_parses_explicit_frames() {
    let resolved = batch_run_resolve_from_args(&args(&[
        "bin",
        "--batch-screenshot",
        "/tmp/out.png",
        "--batch-frames",
        "30",
    ]))
    .unwrap()
    .unwrap();
    assert_eq!(resolved.capture.first_frame, 30);
}

#[test]
fn resolve_parses_sequence_mode() {
    let resolved = batch_run_resolve_from_args(&args(&[
        "bin",
        "--batch-screenshot-sequence",
        "out,3,2",
        "--batch-frames",
        "10",
    ]))
    .unwrap()
    .unwrap();
    assert_eq!(
        resolved.capture,
        CaptureSchedule {
            first_frame: 10,
            stride: 2,
            count: 3,
            output: CaptureOutput::Sequence(PathBuf::from("out")),
        }
    );
    for bad in ["out,0,2", "out,3,0", "out,3", "out,x,2"] {
        assert!(
            batch_run_resolve_from_args(&args(&["bin", "--batch-screenshot-sequence", bad]))
                .is_err(),
            "expected error for '{bad}'"
        );
    }
}

#[test]
fn resolve_rejects_missing_output() {
    assert!(batch_run_resolve_from_args(&args(&["bin", "--batch-screenshot"])).is_err());
    assert!(
        batch_run_resolve_from_args(&args(&["bin", "--batch-screenshot", "--batch-frames"]))
            .is_err()
    );
}

#[test]
fn resolve_rejects_non_png_output() {
    assert!(
        batch_run_resolve_from_args(&args(&["bin", "--batch-screenshot", "/tmp/out.jpg"])).is_err()
    );
}

#[test]
fn resolve_rejects_invalid_frames() {
    for frames in ["0", "abc"] {
        assert!(batch_run_resolve_from_args(&args(&[
            "bin",
            "--batch-screenshot",
            "/tmp/out.png",
            "--batch-frames",
            frames
        ]))
        .is_err());
    }
}

#[test]
fn resolve_rejects_frames_without_screenshot() {
    assert!(batch_run_resolve_from_args(&args(&["bin", "--batch-frames", "30"])).is_err());
}

#[test]
fn resolve_camera_pose() {
    let pose = camera_pose_resolve_from_args(&args(&["bin", "--batch-camera", "30,5,4"]))
        .unwrap()
        .unwrap();
    assert_eq!(
        pose,
        BatchCameraPose {
            yaw_degrees: 30.0,
            pitch_degrees: 5.0,
            distance: 4.0,
            pivot: None
        }
    );
    let pose = camera_pose_resolve_from_args(&args(&["bin", "--batch-camera", "30,5,4,0,1.2,0"]))
        .unwrap()
        .unwrap();
    assert_eq!(pose.pivot, Some([0.0, 1.2, 0.0]));
    assert!(camera_pose_resolve_from_args(&args(&["bin"]))
        .unwrap()
        .is_none());
}

#[test]
fn resolve_rejects_invalid_camera_pose() {
    for value in ["30,5", "a,b,c", "30,5,0", "30,5,-1", "30,5,4,0,1"] {
        assert!(
            camera_pose_resolve_from_args(&args(&["bin", "--batch-camera", value])).is_err(),
            "expected error for '{value}'"
        );
    }
}

#[test]
fn engine_overrides_carry_no_subsystem_flags() {
    let overrides = resolve_engine_cli_overrides(&args(&[
        "bin",
        "--batch-screenshot",
        "/tmp/out.png",
        "--batch-flame-mode",
        "raymarch",
        "--batch-water-probe",
        "/tmp/probe.json",
        "--batch-play",
    ]))
    .unwrap();
    assert!(overrides.batch_run.is_some());
    assert!(overrides.batch_play);
    assert!(overrides.debug_actions.is_empty());
}

#[test]
fn apply_engine_overrides_inserts_the_batch_run_and_leaves_capture_requests_to_actions() {
    let overrides = resolve_engine_cli_overrides(&args(&[
        "bin",
        "--batch-screenshot",
        "/tmp/out.png",
        "--batch-frames",
        "7",
        "--batch-debug-action",
        "dump_wall_probe",
    ]))
    .unwrap();
    let mut world = World::new();
    apply_engine_overrides(&mut world, &mut AssetStorage::new(), &overrides);

    assert_eq!(world.resource::<BatchRun>().capture.first_frame, 7);
    assert!(world.contains_resource::<FlameWallProbeCapture>());
}

#[test]
fn apply_engine_overrides_without_a_batch_run_captures_through_the_event_queue() {
    let overrides =
        resolve_engine_cli_overrides(&args(&["bin", "--batch-debug-action", "dump_wall_probe"]))
            .unwrap();
    let mut world = World::new();
    world.insert_resource(UIEventQueue::new());
    apply_engine_overrides(&mut world, &mut AssetStorage::new(), &overrides);
    assert!(world.get_resource::<BatchRun>().is_none());
    assert!(world.get_resource::<FlameWallProbeCapture>().is_none());

    let events: Vec<UIEvent> = world.resource_mut::<UIEventQueue>().drain().collect();
    assert!(matches!(events[0], UIEvent::CaptureNow(_)));
}

#[test]
fn batch_run_update_orbit_inserts_missing_transform() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert_component(e, FlameEffect::default());
    world.insert_resource(single_run(1));
    world.resource_mut::<BatchRun>().frames_rendered = 1;
    world.insert_resource(BatchFlameOrbit {
        radius: 2.0,
        period_seconds: 4.0,
        initial: None,
    });

    batch_run_update_orbit(&mut world);

    let motion_path = world
        .get_component::<MotionPath>(e)
        .expect("MotionPath should have been inserted");
    assert_eq!(motion_path.center, cgmath::Vector3::new(0.0, 0.0, 0.0));
    assert!((motion_path.radius - 2.0).abs() < 1e-5);
    assert!((motion_path.angular_speed - 2.0 * std::f32::consts::PI / 4.0).abs() < 1e-5);
    drop(motion_path);

    crate::ecs::systems::sync_motion_paths(&mut world);

    let transform = world
        .get_component::<Transform>(e)
        .expect("Transform should have been inserted by sync_motion_paths");
    let offset = compute_orbit_offset(2.0, 4.0, 1.0 / 60.0);
    assert!((transform.translation.x - offset[0]).abs() < 1e-5);
    assert!((transform.translation.z - offset[2]).abs() < 1e-5);
}

#[test]
fn orbit_offset_matches_motion_path_position() {
    use crate::ecs::component::motion_path_position;
    use std::f32::consts::PI;

    let center = cgmath::Vector3::new(1.0, 2.0, 3.0);
    let radius = 1.5;
    let period = 2.0;
    let path = MotionPath {
        center,
        radius,
        angular_speed: 2.0 * PI / period,
        phase_offset: 0.0,
        enabled: true,
    };

    for &t in &[0.0, 0.7, 1.9, 3.3] {
        let mp_pos = motion_path_position(&path, t);
        let offset = compute_orbit_offset(radius, period, t);
        let orbit_pos = cgmath::Vector3::new(
            center.x + offset[0],
            center.y + offset[1],
            center.z + offset[2],
        );
        assert!((mp_pos.x - orbit_pos.x).abs() < 1e-5, "t={t}: x");
        assert!((mp_pos.y - orbit_pos.y).abs() < 1e-5, "t={t}: y");
        assert!((mp_pos.z - orbit_pos.z).abs() < 1e-5, "t={t}: z");
    }
}

#[test]
fn anim_edit_specs_parse_all_forms() {
    let edits = anim_edits_resolve_from_args(&args(&[
        "bin",
        "--batch-anim-edit",
        "debug_keys=42",
        "--batch-anim-edit",
        "key=height@1.5=2.25",
        "--batch-anim-edit",
        "clear",
    ]))
    .unwrap();
    assert_eq!(edits[0], BatchAnimEdit::DebugKeys { seed: 42 });
    assert_eq!(
        edits[1],
        BatchAnimEdit::Key {
            property_type: crate::ecs::component::FlameParam::Height.property_type(),
            time: 1.5,
            value: 2.25
        }
    );
    assert_eq!(edits[2], BatchAnimEdit::Clear);
}

#[test]
fn anim_edit_invalid_specs_are_err() {
    for spec in [
        "debug_keys=abc",
        "key=height@1.5",
        "key=no_such_param@1.0=2.0",
        "key=height@-1.0=2.0",
        "bogus",
    ] {
        assert!(
            anim_edits_resolve_from_args(&args(&["bin", "--batch-anim-edit", spec])).is_err(),
            "{spec} should be rejected"
        );
    }
}

#[test]
fn debug_actions_parse_names_and_view_mode() {
    let actions = debug_actions_resolve_from_args(&args(&[
        "bin",
        "--batch-debug-action",
        "reset_camera",
        "--batch-debug-action",
        "view_mode=normal",
    ]))
    .unwrap();
    assert_eq!(actions[0].name(), "reset_camera");
    assert_eq!(actions[1].name(), "view_mode");
    assert_eq!(format!("{:?}", actions[1]), "ViewMode(Normal)");
    assert!(
        debug_actions_resolve_from_args(&args(&["bin", "--batch-debug-action", "bogus"])).is_err()
    );
    assert!(debug_actions_resolve_from_args(&args(&["bin", "--batch-debug-action"])).is_err());
}

#[test]
fn registry_names_are_unique_and_sorted() {
    let names: Vec<&str> = batch_action_registry()
        .iter()
        .map(|descriptor| descriptor.name)
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names, sorted);
    for expected in ["reset_camera", "view_mode", "spawn_cube"] {
        assert!(names.contains(&expected), "{expected} not registered");
    }
}

#[test]
fn every_registered_name_parses_or_needs_a_value() {
    for descriptor in batch_action_registry() {
        let parsed = (descriptor.parse)(descriptor.name);
        let with_value = (descriptor.parse)(&format!("{}=", descriptor.name))
            .or_else(|| (descriptor.parse)(&format!("{}:", descriptor.name)));
        assert!(
            parsed.is_some() || with_value.is_some(),
            "{} accepts neither its bare name nor a value form",
            descriptor.name
        );
    }
}

#[test]
fn anim_edits_apply_and_dump_reflect_clip_state() {
    let mut world = World::new();
    crate::ecs::systems::spawn_flame(
        &mut world,
        crate::ecs::systems::DEFAULT_FLAME_NAME,
        FlameEffect::default(),
    );
    world.insert_resource(ClipLibrary::new());
    world.insert_resource(TimelineState::new());
    world.insert_resource(crate::ecs::resource::EditHistory::new(10));
    let mut assets = AssetStorage::new();

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[
            BatchAnimEdit::DebugKeys { seed: 7 },
            BatchAnimEdit::Key {
                property_type: crate::ecs::component::FlameParam::Height.property_type(),
                time: 9.0,
                value: 3.5,
            },
        ],
    );
    assert!(
        (world.resource::<TimelineState>().current_time).abs() < 1e-6,
        "key edit must restore timeline time"
    );

    let dump = batch_anim_dump_json(&world);
    let entities = dump["entities"].as_array().unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0]["domain"], "Flame");
    let clip_id = entities[0]["clip_id"]
        .as_u64()
        .expect("flame clip scheduled");
    let clips = dump["clips"].as_array().unwrap();
    let clip = clips
        .iter()
        .find(|c| c["id"].as_u64() == Some(clip_id))
        .expect("clip in dump");
    let curves = clip["scalar_curves"].as_array().unwrap();
    assert_eq!(
        curves.len(),
        crate::ecs::component::FLAME_DOMAIN.channels.len()
    );
    let height = curves
        .iter()
        .find(|c| c["property"] == "height")
        .expect("height curve");
    let keyframes = height["keyframes"].as_array().unwrap();
    assert_eq!(
        keyframes.len(),
        crate::ecs::systems::scalar_clip_systems::DEBUG_KEYS_PER_CURVE + 1
    );
    assert!(keyframes
        .iter()
        .any(|k| (k["time"].as_f64().unwrap() - 9.0).abs() < 1e-6
            && (k["value"].as_f64().unwrap() - 3.5).abs() < 1e-6));
    assert!((clip["duration"].as_f64().unwrap() - 9.0).abs() < 1e-6);
}

#[test]
fn debug_actions_apply_sets_view_mode_and_queues_events() {
    let mut world = World::new();
    world.insert_resource(DebugViewState::default());
    world.insert_resource(UIEventQueue::new());
    batch_apply_debug_actions(
        &mut world,
        &[
            &ViewMode(DebugViewMode::Normal) as &dyn BatchAction,
            &ResetCamera,
        ],
    );
    assert_eq!(
        world.resource::<DebugViewState>().debug_view_mode,
        DebugViewMode::Normal
    );
    let events: Vec<UIEvent> = world.resource_mut::<UIEventQueue>().drain().collect();
    assert!(matches!(events[0], UIEvent::ResetCamera));
}

#[test]
fn sequence_analyze_resolve_dir_only() {
    let args = args(&[
        "bin",
        "--batch-sequence-analyze",
        "data/frames",
        "--batch-sequence-dump",
        "out.json",
    ]);
    let result = batch_sequence_analyze_resolve_from_args(&args)
        .unwrap()
        .unwrap();
    assert_eq!(result.directories.len(), 1);
    assert_eq!(result.directories[0].0, "data/frames");
    assert_eq!(result.directories[0].1, None);
    assert_eq!(result.directories[0].2, None);
    assert_eq!(result.dump_path, "out.json");
}

#[test]
fn sequence_analyze_resolve_dir_with_range() {
    let args = args(&[
        "bin",
        "--batch-sequence-analyze",
        "data/frames,5,10",
        "--batch-sequence-dump",
        "out.json",
    ]);
    let result = batch_sequence_analyze_resolve_from_args(&args)
        .unwrap()
        .unwrap();
    assert_eq!(result.directories.len(), 1);
    assert_eq!(result.directories[0].0, "data/frames");
    assert_eq!(result.directories[0].1, Some(5));
    assert_eq!(result.directories[0].2, Some(10));
}

#[test]
fn sequence_analyze_resolve_multiple_dirs() {
    let args = args(&[
        "bin",
        "--batch-sequence-analyze",
        "data/a",
        "--batch-sequence-analyze",
        "data/b,1,5",
        "--batch-sequence-dump",
        "out.json",
    ]);
    let result = batch_sequence_analyze_resolve_from_args(&args)
        .unwrap()
        .unwrap();
    assert_eq!(result.directories.len(), 2);
    assert_eq!(result.directories[0].0, "data/a");
    assert_eq!(result.directories[1].0, "data/b");
    assert_eq!(result.directories[1].1, Some(1));
    assert_eq!(result.directories[1].2, Some(5));
}

#[test]
fn sequence_analyze_resolve_missing_dump() {
    let args = args(&["bin", "--batch-sequence-analyze", "data/frames"]);
    assert!(batch_sequence_analyze_resolve_from_args(&args).is_err());
}

#[test]
fn sequence_analyze_resolve_invalid_range() {
    let args = args(&[
        "bin",
        "--batch-sequence-analyze",
        "data/frames,abc,10",
        "--batch-sequence-dump",
        "out.json",
    ]);
    assert!(batch_sequence_analyze_resolve_from_args(&args).is_err());
}

#[test]
fn sequence_analyze_resolve_none_without_flag() {
    let args = args(&["bin", "--batch-screenshot", "data/frames"]);
    let result = batch_sequence_analyze_resolve_from_args(&args).unwrap();
    assert!(result.is_none());
}

#[test]
fn sequence_analyze_run_returns_none_without_flag() {
    let result =
        run_sequence_analyze_from_args(args(&["bin", "--batch-screenshot", "data/frames"]));
    assert!(result.is_none());
}

#[test]
fn sequence_analyze_end_to_end() {
    let temp_dir = tempfile::tempdir().unwrap();
    let dir_path = temp_dir.path();
    std::fs::write(dir_path.join("meta.json"), r#"{"fps": 30.0}"#).unwrap();
    for i in 0..3 {
        let value = (i + 1) as u8 * 50;
        write_test_png(&dir_path.join(format!("frame_{:04}.png", i)), 2, 2, value);
    }

    let dump_path = temp_dir.path().join("output.json");
    let result = run_sequence_analyze_from_args(args(&[
        "bin",
        "--batch-sequence-analyze",
        &dir_path.to_string_lossy(),
        "--batch-sequence-dump",
        &dump_path.to_string_lossy(),
    ]))
    .expect("flag present");
    assert!(result.is_ok(), "sequence analysis failed: {:?}", result);

    let content = std::fs::read_to_string(&dump_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let sequences = json["sequences"].as_array().unwrap();
    assert_eq!(sequences.len(), 1);
    let descriptors = &sequences[0]["descriptors"];
    assert!(sequences[0].get("dir").is_some());
    assert!(descriptors.get("f1_width").is_some());
    assert!(descriptors.get("f2_rough").is_some());
    assert!((descriptors["meta"]["fps"].as_f64().unwrap() - 30.0).abs() < 1e-6);
}

#[test]
fn sequence_analyze_range_filter() {
    let temp_dir = tempfile::tempdir().unwrap();
    let dir_path = temp_dir.path();
    std::fs::write(dir_path.join("meta.json"), r#"{"fps": 10.0}"#).unwrap();
    for i in 0..5 {
        let value = (i + 1) as u8 * 30;
        write_test_png(&dir_path.join(format!("frame_{:04}.png", i)), 2, 2, value);
    }

    let dump_path = temp_dir.path().join("output.json");
    let result = run_sequence_analyze_from_args(args(&[
        "bin",
        "--batch-sequence-analyze",
        &format!("{},1,3", dir_path.to_string_lossy()),
        "--batch-sequence-dump",
        &dump_path.to_string_lossy(),
    ]))
    .expect("flag present");
    assert!(result.is_ok(), "sequence analysis failed: {:?}", result);

    let content = std::fs::read_to_string(&dump_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let sequences = json["sequences"].as_array().unwrap();
    assert_eq!(sequences.len(), 1);
    let meta = &sequences[0]["descriptors"]["meta"];
    assert_eq!(meta["frame_count"].as_u64().unwrap(), 3);
}

#[test]
fn sequence_analyze_jpg_error() {
    let temp_dir = tempfile::tempdir().unwrap();
    let dir_path = temp_dir.path();
    std::fs::write(dir_path.join("frame_0001.jpg"), b"fake jpg").unwrap();

    let dump_path = temp_dir.path().join("output.json");
    let result = run_sequence_analyze_from_args(args(&[
        "bin",
        "--batch-sequence-analyze",
        &dir_path.to_string_lossy(),
        "--batch-sequence-dump",
        &dump_path.to_string_lossy(),
    ]))
    .expect("flag present");
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("JPG") || err_msg.contains("jpg"));
}

fn write_test_png(path: &Path, width: u32, height: u32, value: u8) {
    let file = std::fs::File::create(path).unwrap();
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    let pixels = vec![value; (width * height * 3) as usize];
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
}
