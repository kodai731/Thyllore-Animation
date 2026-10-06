use std::collections::BTreeMap;

use cgmath::{Quaternion, Vector3};
use thyllore_anim_core::editable::components::clip::EditableAnimationClip;
use thyllore_anim_core::editable::components::track::BoneTrack;
use thyllore_anim_core::editable::systems::curve_ops::{curve_add_keyframe, curve_sample};
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::motion::components::sampled_pose::SampledPose;
use thyllore_avatar_core::motion::systems::retarget_pose::retarget_from_bones;
use thyllore_math_core::{continuous_euler, decompose, euler_degrees_to_quaternion};

use crate::animation::{BoneId, Skeleton};
use crate::ecs::resource::HumanoidRig;
use crate::ecs::systems::avatar_setup_systems::compute_bone_global_transform;

pub struct StandardBoneKey {
    pub role: HumanoidRole,
    pub euler_degrees: [f32; 3],
    pub hips_translation: Option<[f32; 3]>,
}

pub fn local_pose_to_standard(
    rig: &HumanoidRig,
    skeleton: &Skeleton,
    bone_id: BoneId,
    local_rotation: Quaternion<f32>,
    local_translation: Vector3<f32>,
) -> Option<StandardBoneKey> {
    let (&role, &bone_index) = rig
        .mapping
        .by_role
        .iter()
        .find(|(_, &bone_index)| bone_index as BoneId == bone_id)?;

    let is_hips = role == HumanoidRole::Hips;
    let hips_offset = if is_hips {
        hips_world_offset(skeleton, bone_index, local_translation)
    } else {
        Vector3::new(0.0, 0.0, 0.0)
    };
    let local_rotations = BTreeMap::from([(bone_index, local_rotation)]);
    let pose = retarget_from_bones(&rig.context, &local_rotations, hips_offset);

    Some(StandardBoneKey {
        role,
        euler_degrees: pose.rotations.get(&role).copied()?,
        hips_translation: is_hips.then_some(pose.hips_translation),
    })
}

pub fn convert_clip_to_standard_space(
    clip: &EditableAnimationClip,
    skeleton: &Skeleton,
    rig: &HumanoidRig,
    fps: u32,
) -> EditableAnimationClip {
    let mut standard = clip.clone();
    standard
        .tracks
        .retain(|bone_id, _| !is_mapped_bone(rig, *bone_id));
    for (bone_id, track) in standard.tracks.iter_mut() {
        if let Some(name) = rig.track_names.get(bone_id) {
            track.bone_name = name.clone();
        }
    }

    for (&role, &bone_index) in &rig.mapping.by_role {
        standard.add_track(bone_index as BoneId, role.unity_name().to_string());
    }

    let frame_count = (clip.duration * fps as f32).round() as usize + 1;
    let mut previous_eulers: BTreeMap<HumanoidRole, Vector3<f32>> = BTreeMap::new();
    for frame in 0..frame_count {
        let time = frame as f32 / fps as f32;
        let local_rotations = sample_local_rotations(clip, skeleton, rig, time);
        let hips_offset = sample_hips_offset(clip, skeleton, rig, time);
        let pose = retarget_from_bones(&rig.context, &local_rotations, hips_offset);
        key_standard_pose(&mut standard, rig, &pose, time, &mut previous_eulers);
    }

    standard
}

fn is_mapped_bone(rig: &HumanoidRig, bone_id: BoneId) -> bool {
    rig.mapping
        .by_role
        .values()
        .any(|&bone_index| bone_index as BoneId == bone_id)
}

fn sample_local_rotations(
    clip: &EditableAnimationClip,
    skeleton: &Skeleton,
    rig: &HumanoidRig,
    time: f32,
) -> BTreeMap<usize, Quaternion<f32>> {
    rig.mapping
        .by_role
        .values()
        .map(|&bone_index| {
            let rotation = match clip.tracks.get(&(bone_index as BoneId)) {
                Some(track) => sample_track_rotation(track, time),
                None => decompose(&skeleton.bones[bone_index].local_transform).1,
            };
            (bone_index, rotation)
        })
        .collect()
}

fn sample_track_rotation(track: &BoneTrack, time: f32) -> Quaternion<f32> {
    let euler_degrees = Vector3::new(
        curve_sample(&track.rotation_x, time).unwrap_or(0.0),
        curve_sample(&track.rotation_y, time).unwrap_or(0.0),
        curve_sample(&track.rotation_z, time).unwrap_or(0.0),
    );
    euler_degrees_to_quaternion(&euler_degrees)
}

fn sample_hips_offset(
    clip: &EditableAnimationClip,
    skeleton: &Skeleton,
    rig: &HumanoidRig,
    time: f32,
) -> Vector3<f32> {
    let zero = Vector3::new(0.0, 0.0, 0.0);
    let Some(&hips_bone) = rig.mapping.by_role.get(&HumanoidRole::Hips) else {
        return zero;
    };
    let Some(track) = clip
        .tracks
        .get(&(hips_bone as BoneId))
        .filter(|track| track.has_translation_keyframes())
    else {
        return zero;
    };

    let bind_translation = skeleton.bones[hips_bone].local_transform.w.truncate();
    let translation = Vector3::new(
        curve_sample(&track.translation_x, time).unwrap_or(bind_translation.x),
        curve_sample(&track.translation_y, time).unwrap_or(bind_translation.y),
        curve_sample(&track.translation_z, time).unwrap_or(bind_translation.z),
    );
    hips_world_offset(skeleton, hips_bone, translation)
}

fn hips_world_offset(
    skeleton: &Skeleton,
    hips_bone: usize,
    local_translation: Vector3<f32>,
) -> Vector3<f32> {
    let hips = &skeleton.bones[hips_bone];
    let bind_translation = hips.local_transform.w.truncate();
    let parent_world_rotation = match hips.parent_id {
        Some(parent_id) => {
            decompose(&compute_bone_global_transform(skeleton, parent_id as usize)).1
        }
        None => Quaternion::new(1.0, 0.0, 0.0, 0.0),
    };
    parent_world_rotation * (local_translation - bind_translation)
}

fn key_standard_pose(
    standard: &mut EditableAnimationClip,
    rig: &HumanoidRig,
    pose: &SampledPose,
    time: f32,
    previous_eulers: &mut BTreeMap<HumanoidRole, Vector3<f32>>,
) {
    for (&role, &rotation) in &pose.rotations {
        let Some(&bone_index) = rig.mapping.by_role.get(&role) else {
            continue;
        };
        let Some(track) = standard.tracks.get_mut(&(bone_index as BoneId)) else {
            continue;
        };

        let euler = continuous_euler(Vector3::from(rotation), previous_eulers.get(&role).copied());
        previous_eulers.insert(role, euler);
        curve_add_keyframe(&mut track.rotation_x, time, euler.x);
        curve_add_keyframe(&mut track.rotation_y, time, euler.y);
        curve_add_keyframe(&mut track.rotation_z, time, euler.z);

        if role == HumanoidRole::Hips {
            let [x, y, z] = pose.hips_translation;
            curve_add_keyframe(&mut track.translation_x, time, x);
            curve_add_keyframe(&mut track.translation_y, time, y);
            curve_add_keyframe(&mut track.translation_z, time, z);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_anim_core::editable::components::curve::PropertyCurve;

    use crate::ecs::systems::avatar_setup_systems::find_first_skeleton;
    use crate::ecs::systems::humanoid_bake_systems::bake_to_bone_clip;
    use crate::ecs::systems::humanoid_rig_systems::{
        build_humanoid_rig, copy_test_humanoid_fixture, test_humanoid_world,
    };

    fn assert_curves_match(original: &PropertyCurve, converted: &PropertyCurve, tolerance: f32) {
        for time in [0.0, 0.5, 1.0] {
            let expected = curve_sample(original, time).unwrap_or(0.0);
            let actual = curve_sample(converted, time).unwrap_or(0.0);
            assert!(
                (expected - actual).abs() < tolerance,
                "{:?} at t={}: expected {:.5}, got {:.5}",
                original.property_type,
                time,
                expected,
                actual
            );
        }
    }

    #[test]
    fn standard_space_round_trips_through_bake() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (_, assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets).expect("no skeleton").clone();
        let rig = build_humanoid_rig(&fbx_path, &skeleton, None).expect("test humanoid has no rig");

        let hips_bone = rig.track_bones["Hips"];
        let left_upper_arm_bone = rig.track_bones["LeftUpperArm"];
        let head_bone = rig.track_bones["Head"];

        let mut clip = EditableAnimationClip::new(0, "round_trip".to_string());
        clip.duration = 1.0;
        let hips = clip.add_track(hips_bone, "Hips".to_string());
        curve_add_keyframe(&mut hips.translation_y, 0.0, 0.0);
        curve_add_keyframe(&mut hips.translation_y, 1.0, 0.1);
        curve_add_keyframe(&mut hips.rotation_y, 0.0, 0.0);
        curve_add_keyframe(&mut hips.rotation_y, 1.0, 30.0);
        let left_upper_arm = clip.add_track(left_upper_arm_bone, "LeftUpperArm".to_string());
        curve_add_keyframe(&mut left_upper_arm.rotation_z, 0.0, 0.0);
        curve_add_keyframe(&mut left_upper_arm.rotation_z, 1.0, -60.0);
        let head = clip.add_track(head_bone, "Head".to_string());
        curve_add_keyframe(&mut head.rotation_x, 0.0, 0.0);
        curve_add_keyframe(&mut head.rotation_x, 1.0, 20.0);

        let baked = bake_to_bone_clip(&clip, &skeleton, &rig, 30).unwrap();
        let converted = convert_clip_to_standard_space(&baked, &skeleton, &rig, 30);

        let original_track = |bone_id: BoneId| &clip.tracks[&bone_id];
        let converted_track = |bone_id: BoneId| &converted.tracks[&bone_id];
        assert_curves_match(
            &original_track(hips_bone).translation_y,
            &converted_track(hips_bone).translation_y,
            1e-4,
        );
        assert_curves_match(
            &original_track(hips_bone).rotation_y,
            &converted_track(hips_bone).rotation_y,
            1e-2,
        );
        assert_curves_match(
            &original_track(left_upper_arm_bone).rotation_z,
            &converted_track(left_upper_arm_bone).rotation_z,
            1e-2,
        );
        assert_curves_match(
            &original_track(head_bone).rotation_x,
            &converted_track(head_bone).rotation_x,
            1e-2,
        );
    }

    #[test]
    fn bone_set_key_writes_standard_values_on_a_humanoid_model() {
        use crate::ecs::systems::timeline_systems::process_bone_set_key;
        use thyllore_avatar_core::motion::systems::role_rotation::role_rotation_to_engine;

        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (_, assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets).expect("no skeleton").clone();
        let rig = build_humanoid_rig(&fbx_path, &skeleton, None).expect("test humanoid has no rig");

        let left_upper_arm_bone: BoneId = rig.track_bones["LeftUpperArm"];

        let mut clip_library = crate::ecs::resource::ClipLibrary::new();
        let clip_id: crate::animation::editable::SourceClipId = 1;
        let clip = EditableAnimationClip::new(clip_id, "test".to_string());
        let source = crate::animation::editable::SourceClip::new(clip_id, clip);
        clip_library.source_clips.insert(clip_id, source);

        let mut timeline_state = crate::ecs::resource::TimelineState::new();
        timeline_state.current_clip_id = Some(clip_id);
        timeline_state.current_time = 0.0;

        let mut overrides: std::collections::HashMap<BoneId, crate::animation::BoneLocalPose> =
            std::collections::HashMap::new();

        let left_upper_arm_pose = crate::animation::BoneLocalPose {
            translation: cgmath::Vector3::new(0.0, 0.0, 0.0),
            rotation: role_rotation_to_engine(&rig.context.frame, [0.0, 0.0, -40.0]),
            scale: cgmath::Vector3::new(1.0, 1.0, 1.0),
        };
        overrides.insert(left_upper_arm_bone, left_upper_arm_pose);

        let skirt_bone_id: BoneId = rig.track_bones["Skirt_Front_1"];
        let skirt_pose = crate::animation::BoneLocalPose {
            translation: cgmath::Vector3::new(0.0, 0.0, 0.0),
            rotation: euler_degrees_to_quaternion(&cgmath::Vector3::new(30.0, 0.0, 0.0)),
            scale: cgmath::Vector3::new(1.0, 1.0, 1.0),
        };
        overrides.insert(skirt_bone_id, skirt_pose);

        process_bone_set_key(
            &overrides,
            &mut clip_library,
            &timeline_state,
            &skeleton,
            Some(&rig),
        );

        let clip = clip_library.get(clip_id).unwrap();

        let left_upper_arm_track = clip
            .tracks
            .get(&left_upper_arm_bone)
            .expect("LeftUpperArm track");
        assert_eq!(
            left_upper_arm_track.bone_name, "LeftUpperArm",
            "track name should be role name"
        );

        let rz = curve_sample(&left_upper_arm_track.rotation_z, 0.0).unwrap();
        assert!(
            (rz - (-40.0)).abs() < 1e-2,
            "rotation_z should be ≈ -40, got {:.4}",
            rz
        );
        let rx = curve_sample(&left_upper_arm_track.rotation_x, 0.0).unwrap_or(0.0);
        assert!(rx.abs() < 1e-2, "rotation_x should be ≈ 0, got {:.4}", rx);
        let ry = curve_sample(&left_upper_arm_track.rotation_y, 0.0).unwrap_or(0.0);
        assert!(ry.abs() < 1e-2, "rotation_y should be ≈ 0, got {:.4}", ry);

        assert!(
            left_upper_arm_track.scale_x.keyframes.is_empty(),
            "scale_x should have no keyframes for mapped bone"
        );
        assert!(
            left_upper_arm_track.translation_x.keyframes.is_empty(),
            "translation_x should have no keyframes for non-Hips mapped bone"
        );

        let skirt_track = clip
            .tracks
            .get(&skirt_bone_id)
            .expect("Skirt_Front_1 track");
        let skirt_rx = curve_sample(&skirt_track.rotation_x, 0.0).unwrap();
        assert!(
            (skirt_rx - 30.0).abs() < 1e-2,
            "Skirt_Front_1 rotation_x should be ≈ 30 (local value), got {:.4}",
            skirt_rx
        );
    }
}
