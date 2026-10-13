use std::path::PathBuf;

use super::anim_edit_spec::anim_edits_resolve_from_args;
use super::flags::DEFAULT_SCREENSHOT_FRAME;
use super::test_support::{args, drained_command_names, write_test_png};
use super::*;
use crate::asset::AssetStorage;
use crate::ecs::component::MotionPath;
use crate::ecs::events::UiCommandQueue;
use crate::ecs::resource::{
    BatchEffectOrbit, BatchRun, BatchRunState, CaptureOutput, CaptureSchedule, ClipLibrary,
    DebugViewMode, DebugViewState, FrameClock, ScheduledBatchAction, ScheduledBatchActions,
    TimelineState,
};
use crate::ecs::systems::scalar_clip_systems::test_support::{
    probe_property, spawn_probe, PROBE_DOMAIN, PROBE_LEVEL,
};
use crate::ecs::world::{Transform, World};
use crate::hooks::effect_spawn::EffectSpawnHooks;
use crate::scene::test_support::ProbeOwner;

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
fn apply_engine_overrides_inserts_the_batch_run_and_applies_registered_actions() {
    let overrides = resolve_engine_cli_overrides(&args(&[
        "bin",
        "--batch-screenshot",
        "/tmp/out.png",
        "--batch-frames",
        "7",
        "--batch-debug-action",
        "black_background",
    ]))
    .unwrap();
    let mut world = World::new();
    world.insert_resource(DebugViewState::default());
    apply_engine_overrides(&mut world, &mut AssetStorage::new(), &overrides);

    assert_eq!(world.resource::<BatchRun>().capture.first_frame, 7);
    assert!(world.resource::<DebugViewState>().black_background);
}

#[test]
fn batch_run_update_orbit_inserts_missing_transform() {
    let mut world = World::new();
    world.insert_resource(EffectSpawnHooks::collect().expect("unique effect keys"));
    let e = world.spawn();
    world.insert_component(
        e,
        ProbeOwner {
            position: [0.0, 0.0, 0.0],
            level: 0.5,
        },
    );
    world.insert_resource(FrameClock {
        frame: 1,
        ..FrameClock::fixed(FrameClock::BATCH_DELTA_SECONDS)
    });
    world.insert_resource(BatchEffectOrbit {
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
        "key=probe_level@1.5=2.25",
        "--batch-anim-edit",
        "clear",
    ]))
    .unwrap();
    assert_eq!(edits[0], BatchAnimEdit::DebugKeys { seed: 42 });
    assert_eq!(
        edits[1],
        BatchAnimEdit::Key {
            property_type: probe_property(&PROBE_LEVEL),
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
    spawn_probe(&mut world, "Probe");
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
                property_type: probe_property(&PROBE_LEVEL),
                time: 9.0,
                value: 3.5,
            },
        ],
    );
    assert!(
        (world.resource::<TimelineState>().current_time).abs() < 1e-6,
        "key edit must restore timeline time"
    );

    let dump = batch_anim_dump_json(&world, false);
    let entities = dump["entities"].as_array().unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0]["domain"], PROBE_DOMAIN.name);
    let clip_id = entities[0]["clip_id"]
        .as_u64()
        .expect("probe clip scheduled");
    let clips = dump["clips"].as_array().unwrap();
    let clip = clips
        .iter()
        .find(|c| c["id"].as_u64() == Some(clip_id))
        .expect("clip in dump");
    let curves = clip["scalar_curves"].as_array().unwrap();
    assert_eq!(curves.len(), PROBE_DOMAIN.channels().len());
    let level = curves
        .iter()
        .find(|c| c["property"] == PROBE_LEVEL.cli_name)
        .expect("level curve");
    let keyframes = level["keyframes"].as_array().unwrap();
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
    world.insert_resource(UiCommandQueue::default());
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
    assert_eq!(drained_command_names(&world), vec!["ResetCamera"]);
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

#[test]
fn scheduled_actions_parse_frame_and_action() {
    let scheduled = scheduled_actions_resolve_from_args(&args(&[
        "bin",
        "--batch-debug-action-at",
        "12:timeline_time=0.5",
        "--batch-debug-action-at",
        "3:reset_camera",
    ]))
    .unwrap();

    assert_eq!(
        scheduled,
        vec![
            ScheduledBatchAction {
                frame: 12,
                action: "timeline_time=0.5".to_string(),
            },
            ScheduledBatchAction {
                frame: 3,
                action: "reset_camera".to_string(),
            },
        ]
    );
}

#[test]
fn scheduled_actions_reject_malformed_specs() {
    for spec in ["reset_camera", "x:reset_camera", "5:bogus", "5:"] {
        assert!(
            scheduled_actions_resolve_from_args(&args(&["bin", "--batch-debug-action-at", spec]))
                .is_err(),
            "{spec} must be rejected"
        );
    }
    assert!(
        scheduled_actions_resolve_from_args(&args(&["bin", "--batch-debug-action-at"])).is_err()
    );
}

#[test]
fn scheduled_actions_apply_once_their_frame_is_reached() {
    let mut world = World::new();
    world.insert_resource(UiCommandQueue::default());
    world.insert_resource(FrameClock::fixed(FrameClock::BATCH_DELTA_SECONDS));
    world.insert_resource(ScheduledBatchActions {
        pending: vec![
            ScheduledBatchAction {
                frame: 2,
                action: "timeline_time=0.5".to_string(),
            },
            ScheduledBatchAction {
                frame: 1,
                action: "reset_camera".to_string(),
            },
        ],
    });

    world.resource_mut::<FrameClock>().frame = 1;
    batch_apply_scheduled_actions(&mut world);
    let first_frame_commands = drained_command_names(&world);

    world.resource_mut::<FrameClock>().frame = 2;
    batch_apply_scheduled_actions(&mut world);
    batch_apply_scheduled_actions(&mut world);
    let second_frame_commands = drained_command_names(&world);

    assert_eq!(first_frame_commands, vec!["ResetCamera"]);
    assert_eq!(second_frame_commands, vec!["SetTime(0.5)"]);
    assert!(world.resource::<ScheduledBatchActions>().pending.is_empty());
}

#[test]
fn dump_includes_bone_tracks_when_requested() {
    use crate::ecs::systems::clip_library_register_and_activate;
    use tempfile::tempdir;
    use thyllore_anim_core::editable::{curve_add_keyframe, EditableAnimationClip};

    let tmp = tempdir().unwrap();
    let (world, mut assets) = humanoid_rig_world(tmp.path());

    let mut clip = EditableAnimationClip::new(0, "t".into());
    let track = clip.add_track(
        role_track_bone(&world, "RightUpperArm"),
        "RightUpperArm".into(),
    );
    curve_add_keyframe(&mut track.rotation_z, 0.0, 0.0);
    curve_add_keyframe(&mut track.rotation_z, 1.0, 45.0);

    clip_library_register_and_activate(&mut world.resource_mut::<ClipLibrary>(), &mut assets, clip);

    let dump = batch_anim_dump_json(&world, true);
    let bone_track = &dump["clips"][0]["bone_tracks"][0];
    assert_eq!(bone_track["role"], "RightUpperArm");
    assert_eq!(
        bone_track["curves"]["rot_z"].as_array().map(Vec::len),
        Some(2)
    );

    let dump = batch_anim_dump_json(&world, false);
    assert!(dump["clips"][0].get("bone_tracks").is_none());
}

#[test]
fn anim_edit_parses_clip_specs() {
    use super::anim_edit_spec::anim_edit_parse_spec;

    let edit = anim_edit_parse_spec("new_clip=walk").unwrap();
    match edit {
        BatchAnimEdit::NewClip { name } => assert_eq!(name, "walk"),
        other => panic!("expected NewClip, got {:?}", other),
    }

    let edit = anim_edit_parse_spec("template=/tmp/a.anim.json").unwrap();
    match edit {
        BatchAnimEdit::Template { path } => assert_eq!(path.to_str().unwrap(), "/tmp/a.anim.json"),
        other => panic!("expected Template, got {:?}", other),
    }

    let edit = anim_edit_parse_spec("save=/tmp/b.anim.json").unwrap();
    match edit {
        BatchAnimEdit::Save { path } => assert_eq!(path.to_str().unwrap(), "/tmp/b.anim.json"),
        other => panic!("expected Save, got {:?}", other),
    }
}

#[test]
fn copilot_extend_parses_role_axis_time_frames() {
    use super::anim_edit_spec::anim_edit_parse_spec;
    use super::BoneAxis;

    let edit = anim_edit_parse_spec("copilot_extend=Hips.y@1.5,30").unwrap();
    match edit {
        BatchAnimEdit::CopilotExtend {
            bone_name,
            axis,
            time,
            frames,
        } => {
            assert_eq!(bone_name, "Hips");
            assert_eq!(axis, BoneAxis::RotationY);
            assert_eq!(time, 1.5);
            assert_eq!(frames, 30);
        }
        other => panic!("expected CopilotExtend, got {:?}", other),
    }

    assert!(anim_edit_parse_spec("copilot_extend=Hips.tx@1.0,30").is_err());
    assert!(anim_edit_parse_spec("copilot_extend=Hips.x@1.0").is_err());
}

#[test]
fn template_then_save_round_trips_a_clip() {
    use crate::ecs::systems::humanoid_bake_systems::new_empty_clip;
    use tempfile::tempdir;

    let tmp = tempdir().unwrap();
    let (mut world, mut assets) = humanoid_rig_world(tmp.path());

    let template_path = tmp.path().join("template.anim.json");
    let save_path = tmp.path().join("saved.anim.json");

    let clip = new_empty_clip("walk");
    crate::scene::save_animation_clip(&template_path, &clip).unwrap();

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[BatchAnimEdit::Template {
            path: template_path.clone(),
        }],
    );

    let current_id = world.resource::<TimelineState>().current_clip_id;
    assert!(current_id.is_some());

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[BatchAnimEdit::Save {
            path: save_path.clone(),
        }],
    );

    let loaded_template = crate::scene::load_animation_clip(&template_path).unwrap();
    let loaded_saved = crate::scene::load_animation_clip(&save_path).unwrap();

    assert_eq!(loaded_template.tracks.len(), loaded_saved.tracks.len());
}

#[test]
fn template_replaces_a_same_named_bone_clip_with_a_playable_asset() {
    use tempfile::tempdir;

    let mut world = World::new();
    world.insert_resource(ClipLibrary::new());
    world.insert_resource(TimelineState::default());

    let mut assets = AssetStorage::new();
    let tmp = tempdir().unwrap();

    let template_path = tmp.path().join("template.anim.json");

    let clip = thyllore_anim_core::editable::EditableAnimationClip::new(99, "walk".to_string());
    crate::scene::save_animation_clip(&template_path, &clip).unwrap();

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[BatchAnimEdit::Template {
            path: template_path.clone(),
        }],
    );

    {
        let lib = world.resource::<ClipLibrary>();
        let names: Vec<_> = lib.source_clips.values().map(|s| s.name()).collect();
        assert_eq!(names, vec!["walk"]);

        let source_id = *lib.source_clips.keys().next().unwrap();
        let asset_id = *lib.source_to_asset_id.get(&source_id).unwrap();
        assert!(assets.animation_clips.contains_key(&asset_id));
    }

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[BatchAnimEdit::Template {
            path: template_path.clone(),
        }],
    );

    {
        let lib = world.resource::<ClipLibrary>();
        assert_eq!(lib.source_clips.len(), 1);
        let names: Vec<_> = lib.source_clips.values().map(|s| s.name()).collect();
        assert_eq!(names, vec!["walk"]);

        let source_id = *lib.source_clips.keys().next().unwrap();
        let asset_id = *lib.source_to_asset_id.get(&source_id).unwrap();
        assert!(assets.animation_clips.contains_key(&asset_id));
    }
}

fn humanoid_rig_world(dir: &std::path::Path) -> (World, AssetStorage) {
    use crate::ecs::resource::HumanoidRigState;
    use crate::ecs::systems::humanoid_rig_systems::{
        build_humanoid_rig, copy_test_humanoid_fixture, test_humanoid_world,
    };

    let (fbx_path, _) = copy_test_humanoid_fixture(dir);
    let (mut world, assets) = test_humanoid_world(&fbx_path);
    let skeleton = &assets.skeletons.values().next().unwrap().skeleton;
    let rig = build_humanoid_rig(&fbx_path, skeleton, None).expect("test humanoid has no rig");
    world.resource_mut::<HumanoidRigState>().rig = Some(rig);
    world.insert_resource(ClipLibrary::new());
    world.insert_resource(TimelineState::default());
    (world, assets)
}

fn role_track_bone(world: &World, role_name: &str) -> thyllore_anim_core::BoneId {
    let state = world.resource::<crate::ecs::resource::HumanoidRigState>();
    let rig = state.rig.as_ref().unwrap();
    rig.track_bones[role_name]
}

#[test]
fn key_edit_addresses_role_curve() {
    use super::anim_edit_spec::anim_edit_parse_spec;
    use tempfile::tempdir;

    let tmp = tempdir().unwrap();
    let (mut world, mut assets) = humanoid_rig_world(tmp.path());

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[anim_edit_parse_spec("new_clip=t").unwrap()],
    );

    let clip_id = world.resource::<TimelineState>().current_clip_id.unwrap();

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[anim_edit_parse_spec("key=Head.x@0.5=20").unwrap()],
    );

    let lib = world.resource::<ClipLibrary>();
    let clip = lib.get(clip_id).unwrap();

    let track = clip.get_track(role_track_bone(&world, "Head")).unwrap();
    assert_eq!(track.rotation_x.keyframes.len(), 1);
    let kf = &track.rotation_x.keyframes[0];
    assert!((kf.time - 0.5).abs() < f32::EPSILON);
    assert!((kf.value - 20.0).abs() < f32::EPSILON);

    let lib = world.resource::<ClipLibrary>();
    assert!(lib.dirty_sources.contains(&clip_id));
}

#[test]
fn key_edit_rejects_role_on_bone_clip() {
    use super::anim_edit_spec::anim_edit_parse_spec;

    let mut world = World::new();
    world.insert_resource(ClipLibrary::new());
    world.insert_resource(TimelineState::default());

    let mut assets = AssetStorage::new();

    let clip = thyllore_anim_core::editable::EditableAnimationClip::new(1, "bone".to_string());
    let id = crate::ecs::systems::clip_library_register_and_activate(
        &mut world.resource_mut::<ClipLibrary>(),
        &mut assets,
        clip,
    );
    world.resource_mut::<TimelineState>().current_clip_id = Some(id);

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[anim_edit_parse_spec("key=Head.x@0.5=20").unwrap()],
    );

    let lib = world.resource::<ClipLibrary>();
    let clip = lib.get(id).unwrap();
    assert_eq!(clip.tracks.len(), 0);
}

#[test]
fn role_key_rejects_translation_on_non_hips() {
    use super::anim_edit_spec::anim_edit_parse_spec;

    let result = anim_edit_parse_spec("key=Head.tx@0=1");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("tx"));
}

#[test]
fn bone_key_rejects_translation_on_a_non_hips_role() {
    use super::anim_edit_spec::anim_edit_parse_spec;

    assert!(anim_edit_parse_spec("key=Head.tx@0=1").is_err());
    assert!(anim_edit_parse_spec("key=Skirt_Front_1.tx@0=1").is_ok());
}

#[test]
fn bone_key_addresses_an_unmapped_bone() {
    use super::anim_edit_spec::anim_edit_parse_spec;
    use tempfile::tempdir;

    let tmp = tempdir().unwrap();
    let (mut world, mut assets) = humanoid_rig_world(tmp.path());

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[anim_edit_parse_spec("new_clip=t").unwrap()],
    );

    let clip_id = world.resource::<TimelineState>().current_clip_id.unwrap();

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[anim_edit_parse_spec("key=Skirt_Front_1.x@1=45").unwrap()],
    );

    let lib = world.resource::<ClipLibrary>();
    let clip = lib.get(clip_id).unwrap();
    let track = clip
        .get_track(role_track_bone(&world, "Skirt_Front_1"))
        .unwrap();
    assert_eq!(track.rotation_x.keyframes.len(), 1);
    let kf = &track.rotation_x.keyframes[0];
    assert!((kf.time - 1.0).abs() < f32::EPSILON);
    assert!((kf.value - 45.0).abs() < f32::EPSILON);
}

#[test]
fn role_key_extends_duration() {
    use super::anim_edit_spec::anim_edit_parse_spec;
    use tempfile::tempdir;

    let tmp = tempdir().unwrap();
    let (mut world, mut assets) = humanoid_rig_world(tmp.path());

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[anim_edit_parse_spec("new_clip=t").unwrap()],
    );

    let clip_id = world.resource::<TimelineState>().current_clip_id.unwrap();

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[anim_edit_parse_spec("key=Hips.ty@3=0.1").unwrap()],
    );

    let lib = world.resource::<ClipLibrary>();
    let clip = lib.get(clip_id).unwrap();
    assert!((clip.duration - 3.0).abs() < f32::EPSILON);
}

#[test]
fn role_key_keeps_earlier_keys_on_the_same_curve() {
    use super::anim_edit_spec::anim_edit_parse_spec;
    use tempfile::tempdir;

    let tmp = tempdir().unwrap();
    let (mut world, mut assets) = humanoid_rig_world(tmp.path());

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[anim_edit_parse_spec("new_clip=t").unwrap()],
    );

    let clip_id = world.resource::<TimelineState>().current_clip_id.unwrap();

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[anim_edit_parse_spec("key=LeftUpperArm.z@0=0").unwrap()],
    );

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[anim_edit_parse_spec("key=LeftUpperArm.z@1=-90").unwrap()],
    );

    let lib = world.resource::<ClipLibrary>();
    let clip = lib.get(clip_id).unwrap();
    let track = clip
        .get_track(role_track_bone(&world, "LeftUpperArm"))
        .unwrap();
    assert_eq!(track.rotation_z.keyframes.len(), 2);
    assert!((track.rotation_z.keyframes[0].time - 0.0).abs() < f32::EPSILON);
    assert!((track.rotation_z.keyframes[0].value - 0.0).abs() < f32::EPSILON);
    assert!((track.rotation_z.keyframes[1].time - 1.0).abs() < f32::EPSILON);
    assert!((track.rotation_z.keyframes[1].value - (-90.0)).abs() < f32::EPSILON);
}

#[test]
fn role_key_builds_the_rig_before_resolving_names() {
    use super::anim_edit_spec::anim_edit_parse_spec;
    use crate::ecs::systems::humanoid_rig_systems::{
        build_humanoid_rig, copy_test_humanoid_fixture, test_humanoid_world,
    };
    use tempfile::tempdir;

    let tmp = tempdir().unwrap();
    let (fbx_path, _) = copy_test_humanoid_fixture(tmp.path());
    let (mut world, mut assets) = test_humanoid_world(&fbx_path);
    world.insert_resource(ClipLibrary::new());
    world.insert_resource(TimelineState::default());

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[anim_edit_parse_spec("new_clip=t").unwrap()],
    );

    let clip_id = world.resource::<TimelineState>().current_clip_id.unwrap();

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[anim_edit_parse_spec("key=Head.x@1=20").unwrap()],
    );

    let state = world.resource::<crate::ecs::resource::HumanoidRigState>();
    let rig = state.rig.as_ref().unwrap();
    let bone_id = rig.track_bones["Head"];

    let lib = world.resource::<ClipLibrary>();
    let clip = lib.get(clip_id).unwrap();
    let track = clip.get_track(bone_id).unwrap();
    assert_eq!(track.rotation_x.keyframes.len(), 1);
    let kf = &track.rotation_x.keyframes[0];
    assert!((kf.time - 1.0).abs() < f32::EPSILON);
    assert!((kf.value - 20.0).abs() < f32::EPSILON);
}

#[test]
fn compose_edit_keys_every_touched_role_from_the_pose_table() {
    use super::anim_edit_spec::anim_edit_parse_spec;
    use tempfile::tempdir;

    let tmp = tempdir().unwrap();
    let (mut world, mut assets) = humanoid_rig_world(tmp.path());

    batch_apply_anim_edits(
        &mut world,
        &mut assets,
        &[
            anim_edit_parse_spec("new_clip=p").unwrap(),
            anim_edit_parse_spec("compose=punch,side=left").unwrap(),
        ],
    );

    let clip_id = world.resource::<TimelineState>().current_clip_id.unwrap();
    let lib = world.resource::<ClipLibrary>();
    let clip = lib.get(clip_id).unwrap();
    assert_eq!(clip.tracks.len(), 10);
    assert!((clip.duration - 2.0).abs() < 1e-6);

    let left_upper = clip
        .get_track(role_track_bone(&world, "LeftUpperArm"))
        .unwrap();
    let punch_key = &left_upper.rotation_y.keyframes[2];
    assert!((punch_key.time - 0.55).abs() < 1e-6);
    assert!((punch_key.value - 85.0).abs() < 1e-6);
}

#[test]
fn helm_compose_motion_registers_a_new_clip_and_schedules_it_on_the_target() {
    use crate::ecs::component::ClipSchedule;
    use crate::ecs::events::UiCommand;
    use crate::ecs::systems::helm::HelmCommand;
    use crate::ecs::systems::motion_seed_systems::composed_clip_name;
    use crate::vulkanr::resource::graphics_resource::GraphicsResources;
    use tempfile::tempdir;
    use thyllore_avatar_core::motion::seed::components::motion_spec::MotionSpec;

    let tmp = tempdir().unwrap();
    let (mut world, mut assets) = humanoid_rig_world(tmp.path());
    world.insert_resource(crate::ecs::resource::BakedHumanoidClips::default());
    let target = world.spawn();
    world.insert_component(target, ClipSchedule::default());
    let spec = MotionSpec::parse("wave,side=left,count=2").unwrap();

    Box::new(HelmCommand::ComposeMotion {
        entity: target,
        spec: spec.clone(),
        start_time: 0.5,
    })
    .apply(&mut world, &mut assets, &GraphicsResources::default());

    let clip_id = world.resource::<TimelineState>().current_clip_id.unwrap();
    let lib = world.resource::<ClipLibrary>();
    let clip = lib.get(clip_id).unwrap();
    assert_eq!(clip.name, composed_clip_name(&spec));
    assert_eq!(clip.name, "left_wave");
    assert!(
        clip.get_track(role_track_bone(&world, "LeftUpperArm"))
            .is_some_and(|track| track.has_rotation_keyframes()),
        "the left side mirrors the right-defined wave"
    );
    let schedule = world.get_component::<ClipSchedule>(target).unwrap();
    assert_eq!(schedule.instances.len(), 1);
    assert_eq!(schedule.instances[0].source_id, clip_id);
    assert!((schedule.instances[0].start_time - 0.5).abs() < 1e-6);
    assert!(
        world
            .resource::<crate::ecs::resource::BakedHumanoidClips>()
            .scan_requested
    );
}

#[test]
fn compose_edit_rejects_an_unknown_motion_at_parse_time() {
    use super::anim_edit_spec::anim_edit_parse_spec;
    assert!(anim_edit_parse_spec("compose=").is_err());
    assert!(anim_edit_parse_spec("compose=punch,count=0").is_err());
    assert!(anim_edit_parse_spec("compose=punch,count=2").is_ok());
}
