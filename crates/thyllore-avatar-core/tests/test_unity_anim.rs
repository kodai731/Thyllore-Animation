#![cfg(test)]

mod support;

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use thyllore_anim_core::editable::components::clip::EditableAnimationClip;
use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::motion::systems::humanoid_pose_sampler::sample_humanoid_pose;
use thyllore_avatar_core::muscle::components::unity_muscle_table::default_unity_muscle_table;
use thyllore_avatar_core::muscle::systems::muscle_curves_sampler::muscle_curves_from_clip;
use thyllore_avatar_core::muscle::systems::unity_anim_writer::{unity_anim_text, write_unity_anim};
use thyllore_avatar_core::muscle::systems::unity_muscle_conversion::to_unity_muscles;

fn mapping() -> HumanoidMapping {
    support::test_humanoid::build_ctx().0.mapping.clone()
}

#[test]
fn wave_curves_follow_the_pose() {
    let mapping = mapping();
    let table = default_unity_muscle_table();
    let sample_rate: u32 = 30;

    let mut clip = support::role_clips::wave();
    clip.id = 1;
    remap_tracks(&mut clip, &mapping);

    let curves = muscle_curves_from_clip(&clip, &mapping, sample_rate, table);

    for (frame_idx, frame) in curves.frames.iter().enumerate() {
        let time = curves.times[frame_idx];
        let pose = sample_humanoid_pose(&clip, &mapping, time);
        let expected = to_unity_muscles(&pose, table);

        for (i, (actual, exp)) in frame.iter().zip(expected.iter()).enumerate() {
            assert!(
                (actual - exp).abs() < 1e-6,
                "frame {} muscle {}: actual={:.8}, expected={:.8}",
                frame_idx,
                i,
                actual,
                exp
            );
        }
    }

    let rest = curves.frames[0][48];
    let mut deviates = false;
    for frame in &curves.frames {
        if (frame[48] - rest).abs() >= 0.1 {
            deviates = true;
            break;
        }
    }
    assert!(
        deviates,
        "expected some frame where Right Arm Down-Up deviates from rest by >= 0.1"
    );
}

#[test]
fn anim_text_has_95_float_and_editor_curves() {
    let mapping = mapping();
    let table = default_unity_muscle_table();
    let sample_rate: u32 = 30;

    let mut clip = support::role_clips::wave();
    clip.id = 1;
    remap_tracks(&mut clip, &mapping);

    let curves = muscle_curves_from_clip(&clip, &mapping, sample_rate, table);
    let text = unity_anim_text("test_wave", &curves, table, true);

    let attribute_count = text.matches("attribute: Left Arm Down-Up").count();
    assert_eq!(
        attribute_count, 2,
        "expected 2 occurrences of 'attribute: Left Arm Down-Up' (m_FloatCurves + m_EditorCurves), got {}",
        attribute_count
    );

    let classid_count = text.matches("classID: 95").count();
    assert_eq!(
        classid_count, 190,
        "expected 190 occurrences of 'classID: 95' (2 * 95 muscles), got {}",
        classid_count
    );

    assert!(
        text.contains("m_LoopTime: 1"),
        "expected m_LoopTime: 1 in output"
    );

    assert!(
        text.contains("m_StopTime: 2.8"),
        "expected m_StopTime: 2.8 in output"
    );
}

#[test]
fn wave_slopes_are_finite_differences() {
    let mapping = mapping();
    let table = default_unity_muscle_table();
    let sample_rate: u32 = 30;

    let mut clip = support::role_clips::wave();
    clip.id = 1;
    remap_tracks(&mut clip, &mapping);

    let curves = muscle_curves_from_clip(&clip, &mapping, sample_rate, table);
    let text = unity_anim_text("test_wave", &curves, table, true);

    let right_arm_idx = table
        .muscles
        .iter()
        .position(|m| m.attribute == "Right Arm Down-Up")
        .unwrap();

    let n = curves.times.len();
    let second_last = n - 2;
    let expected_slope = (curves.frames[n - 1][right_arm_idx]
        - curves.frames[second_last][right_arm_idx])
        / (curves.times[n - 1] - curves.times[second_last]);

    let curve_blocks: Vec<&str> = text.split("\n  - curve:\n").collect();
    let mut out_slopes = Vec::new();
    let mut found = false;
    for block in &curve_blocks {
        if !block.contains("attribute: Right Arm Down-Up") {
            continue;
        }
        if found {
            break;
        }
        found = true;
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("        outSlope: ") {
                let value: f32 = rest.trim().parse().unwrap();
                out_slopes.push(value);
            }
        }
    }

    assert_eq!(
        out_slopes.len(),
        n,
        "expected {} keyframes, got {}",
        n,
        out_slopes.len()
    );

    let actual_out_slope = out_slopes[second_last];
    assert!(
        (actual_out_slope - expected_slope).abs() < 1e-4,
        "second-last key outSlope: actual={:.8}, expected={:.8} (finite difference from last key)",
        actual_out_slope,
        expected_slope
    );
}

fn remap_tracks(clip: &mut EditableAnimationClip, mapping: &HumanoidMapping) {
    let old_tracks: Vec<_> = clip.tracks.drain().collect();
    for (_old_id, mut track) in old_tracks {
        let role = HumanoidRole::from_unity_name(&track.bone_name)
            .expect("wave clip contains a track whose bone name is not a known role");
        let bone_id = mapping.by_role[&role] as u32;
        track.bone_id = bone_id;
        clip.tracks.insert(bone_id, track);
    }
}

#[test]
#[ignore]
fn dump_unity_verify_inputs() {
    let out_dir: PathBuf = env::var("UNITY_MUSCLE_VERIFY_DIR")
        .map(PathBuf::from)
        .expect("UNITY_MUSCLE_VERIFY_DIR not set");
    fs::create_dir_all(&out_dir).unwrap();

    let mapping = mapping();
    let table = default_unity_muscle_table();
    let sample_rate: u32 = 30;

    let mut clips: Vec<serde_json::Value> = Vec::new();

    for (name, mut clip) in [
        ("wave", support::role_clips::wave()),
        ("bow", support::role_clips::bow()),
    ] {
        remap_tracks(&mut clip, &mapping);
        let curves = muscle_curves_from_clip(&clip, &mapping, sample_rate, table);
        write_unity_anim(
            &out_dir.join(format!("{name}.anim")),
            name,
            &curves,
            table,
            false,
        )
        .unwrap();

        let frames: Vec<serde_json::Value> = curves
            .frames
            .iter()
            .enumerate()
            .map(|(frame_idx, frame)| {
                let time = curves.times[frame_idx];
                let pose = sample_humanoid_pose(&clip, &mapping, time);
                let rotations: BTreeMap<String, [f32; 3]> = pose
                    .rotations
                    .iter()
                    .map(|(role, euler_degrees)| (role.unity_name().to_string(), *euler_degrees))
                    .collect();
                let muscles: Vec<f32> = frame.to_vec();
                serde_json::json!({
                    "time": time,
                    "rotations": rotations,
                    "muscles": muscles,
                })
            })
            .collect();

        clips.push(serde_json::json!({
            "name": name,
            "frames": frames,
        }));
    }

    let output = serde_json::json!({ "clips": clips });
    fs::write(
        out_dir.join("verify_poses.json"),
        serde_json::to_string_pretty(&output).unwrap(),
    )
    .unwrap();
}
