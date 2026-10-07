#![cfg(test)]

mod support;

use cgmath::InnerSpace;

use thyllore_anim_core::editable::components::clip::EditableAnimationClip;
use thyllore_anim_core::editable::systems::curve_ops::{curve_add_keyframe, curve_sample};

use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::motion::systems::bake::bake_humanoid_clip;
use thyllore_avatar_core::motion::systems::humanoid_pose_sampler::sample_humanoid_pose;
use thyllore_avatar_core::motion::systems::retarget_pose::{
    build_retarget_context, retarget_from_bones, retarget_to_bones,
};
use thyllore_avatar_core::motion::systems::role_rotation::role_rotation_to_thyllore;

use support::test_humanoid::build_ctx;

fn skirt_front_1_bone_index(
    bones: &[thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput],
) -> usize {
    bones
        .iter()
        .position(|b| b.name == "Skirt_Front_1")
        .expect("Skirt_Front_1 not found in skeleton")
}

#[test]
fn test_humanoid_bake_is_the_role_rotation() {
    let (ctx, bones) = build_ctx();

    let left_upper_arm_idx = *ctx
        .mapping
        .by_role
        .get(&HumanoidRole::LeftUpperArm)
        .expect("LeftUpperArm not in mapping");
    let head_idx = *ctx
        .mapping
        .by_role
        .get(&HumanoidRole::Head)
        .expect("Head not in mapping");
    let skirt_idx = skirt_front_1_bone_index(&bones);

    let mut clip = EditableAnimationClip::new(1, "test".to_string());
    clip.duration = 2.0;

    {
        let track = clip.add_track(left_upper_arm_idx as u32, "LeftUpperArm".to_string());
        curve_add_keyframe(&mut track.rotation_z, 0.0, 0.0);
        curve_add_keyframe(&mut track.rotation_z, 1.0, -90.0);
    }

    {
        let head_track = clip.add_track(head_idx as u32, "Head".to_string());
        curve_add_keyframe(&mut head_track.rotation_x, 0.0, 20.0);
        curve_add_keyframe(&mut head_track.rotation_y, 0.0, -30.0);
    }

    {
        let skirt_track = clip.add_track(skirt_idx as u32, "Skirt_Front_1".to_string());
        curve_add_keyframe(&mut skirt_track.rotation_x, 0.0, 45.0);
    }

    let baked = bake_humanoid_clip(&ctx, &clip, 30);

    let left_upper_arm_curve = baked
        .bone_rotations
        .get(&left_upper_arm_idx)
        .expect("LeftUpperArm bone curve not found");
    let head_curve = baked
        .bone_rotations
        .get(&head_idx)
        .expect("Head bone curve not found");

    assert!(
        !baked.bone_rotations.contains_key(&skirt_idx),
        "Skirt_Front_1 should not be in bone_rotations"
    );

    for frame_idx in 0..baked.frame_times.len() {
        let time = baked.frame_times[frame_idx];

        let lua_track = clip
            .get_track(left_upper_arm_idx.try_into().unwrap())
            .expect("LeftUpperArm track");
        let expected_euler = [
            curve_sample(&lua_track.rotation_x, time).unwrap_or(0.0),
            curve_sample(&lua_track.rotation_y, time).unwrap_or(0.0),
            curve_sample(&lua_track.rotation_z, time).unwrap_or(0.0),
        ];
        let expected_q = role_rotation_to_thyllore(&ctx.frame, expected_euler);
        let actual_q = left_upper_arm_curve[frame_idx];
        let dot = expected_q.dot(actual_q).abs();
        assert!(
            (dot - 1.0).abs() < 1e-4,
            "LeftUpperArm frame {}: dot={:.6}, expected={:?}, actual={:?}",
            frame_idx,
            dot,
            expected_q,
            actual_q
        );

        let head_track = clip
            .get_track(head_idx.try_into().unwrap())
            .expect("Head track");
        let head_expected_euler = [
            curve_sample(&head_track.rotation_x, time).unwrap_or(0.0),
            curve_sample(&head_track.rotation_y, time).unwrap_or(0.0),
            curve_sample(&head_track.rotation_z, time).unwrap_or(0.0),
        ];
        let head_expected_q = role_rotation_to_thyllore(&ctx.frame, head_expected_euler);
        let head_actual_q = head_curve[frame_idx];
        let head_dot = head_expected_q.dot(head_actual_q).abs();
        assert!(
            (head_dot - 1.0).abs() < 1e-4,
            "Head frame {}: dot={:.6}, expected={:?}, actual={:?}",
            frame_idx,
            head_dot,
            head_expected_q,
            head_actual_q
        );
    }
}

#[test]
fn test_humanoid_retarget_from_bones_recovers_the_clip() {
    let (ctx, bones) = build_ctx();

    let left_upper_arm_idx = *ctx
        .mapping
        .by_role
        .get(&HumanoidRole::LeftUpperArm)
        .expect("LeftUpperArm not in mapping");
    let head_idx = *ctx
        .mapping
        .by_role
        .get(&HumanoidRole::Head)
        .expect("Head not in mapping");
    let skirt_idx = skirt_front_1_bone_index(&bones);

    let mut clip = EditableAnimationClip::new(1, "test".to_string());
    clip.duration = 2.0;

    {
        let track = clip.add_track(left_upper_arm_idx as u32, "LeftUpperArm".to_string());
        curve_add_keyframe(&mut track.rotation_z, 0.0, 0.0);
        curve_add_keyframe(&mut track.rotation_z, 1.0, -90.0);
    }

    {
        let head_track = clip.add_track(head_idx as u32, "Head".to_string());
        curve_add_keyframe(&mut head_track.rotation_x, 0.0, 20.0);
        curve_add_keyframe(&mut head_track.rotation_y, 0.0, -30.0);
    }

    {
        let skirt_track = clip.add_track(skirt_idx as u32, "Skirt_Front_1".to_string());
        curve_add_keyframe(&mut skirt_track.rotation_x, 0.0, 45.0);
    }

    let sampled = sample_humanoid_pose(&clip, &ctx.mapping, 0.5);
    let retargeted = retarget_to_bones(&ctx, &sampled);
    let recovered = retarget_from_bones(&ctx, &retargeted.local_rotations, retargeted.hips_offset);

    let original_lua = sampled
        .rotations
        .get(&HumanoidRole::LeftUpperArm)
        .expect("LeftUpperArm in sampled");
    let recovered_lua = recovered
        .rotations
        .get(&HumanoidRole::LeftUpperArm)
        .expect("LeftUpperArm in recovered");
    for axis in 0..3 {
        assert!(
            (original_lua[axis] - recovered_lua[axis]).abs() < 1e-2,
            "LeftUpperArm axis {}: original={:.4}, recovered={:.4}",
            axis,
            original_lua[axis],
            recovered_lua[axis]
        );
    }

    let original_head = sampled
        .rotations
        .get(&HumanoidRole::Head)
        .expect("Head in sampled");
    let recovered_head = recovered
        .rotations
        .get(&HumanoidRole::Head)
        .expect("Head in recovered");
    for axis in 0..3 {
        assert!(
            (original_head[axis] - recovered_head[axis]).abs() < 1e-2,
            "Head axis {}: original={:.4}, recovered={:.4}",
            axis,
            original_head[axis],
            recovered_head[axis]
        );
    }
}
