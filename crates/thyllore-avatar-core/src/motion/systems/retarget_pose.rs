use std::collections::BTreeMap;

use cgmath::{InnerSpace, Matrix, Matrix3, One, Quaternion, Rotation, Vector3, Zero};

use crate::humanoid::components::humanoid_frame::HumanoidFrame;
use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::rest_pose::RestPose;
use crate::humanoid::components::role::HumanoidRole;
use crate::motion::components::retarget_context::{RetargetContext, RetargetedPose};
use crate::motion::components::retarget_skeleton::RetargetSkeleton;
use crate::motion::components::sampled_pose::SampledPose;

use super::role_rotation::{role_rotation_to_thyllore, thyllore_rotation_to_role};
use super::tpose_basis::compute_tpose_world_rotations;

pub fn build_retarget_context(
    skeleton: &RetargetSkeleton,
    mapping: &HumanoidMapping,
    frame: &HumanoidFrame,
    rest_pose: RestPose,
) -> Option<RetargetContext> {
    let hips_idx = mapping.by_role.get(&HumanoidRole::Hips)?;
    let left_foot_idx = *mapping.by_role.get(&HumanoidRole::LeftFoot)?;
    let right_foot_idx = *mapping.by_role.get(&HumanoidRole::RightFoot)?;

    let hips_pos = skeleton.bones[*hips_idx].world_position;
    let left_foot_pos = skeleton.bones[left_foot_idx].world_position;
    let right_foot_pos = skeleton.bones[right_foot_idx].world_position;

    let up = Vector3::from(frame.up);
    let lower_foot_y = left_foot_pos.dot(up).min(right_foot_pos.dot(up));
    let hips_height = hips_pos.dot(up) - lower_foot_y;

    let tpose_world = compute_tpose_world_rotations(skeleton, mapping, frame, rest_pose);

    Some(RetargetContext {
        skeleton: skeleton.clone(),
        mapping: mapping.clone(),
        frame: *frame,
        tpose_world,
        hips_height,
    })
}

pub fn retarget_to_bones(ctx: &RetargetContext, pose: &SampledPose) -> RetargetedPose {
    let mut local_rotations = BTreeMap::new();

    for (&role, &bone_idx) in &ctx.mapping.by_role {
        let euler = pose.rotations.get(&role).copied().unwrap_or([0.0; 3]);
        let r_i = role_rotation_to_thyllore(&ctx.frame, euler);

        let parent_rot = match ctx.skeleton.bones[bone_idx].parent {
            Some(parent_idx) => ctx.tpose_world[parent_idx],
            None => Quaternion::one(),
        };

        let t_parent_inv = parent_rot.invert();
        let t_i = ctx.tpose_world[bone_idx];
        let local_i = t_parent_inv * r_i * t_i;

        local_rotations.insert(bone_idx, local_i);
    }

    let m = Matrix3::from_cols(
        Vector3::from(ctx.frame.right),
        Vector3::from(ctx.frame.up),
        Vector3::from(ctx.frame.forward),
    );
    let hips_translation = Vector3::new(
        pose.hips_translation[0],
        pose.hips_translation[1],
        pose.hips_translation[2],
    );
    let hips_offset = m * hips_translation * ctx.hips_height;

    RetargetedPose {
        local_rotations,
        hips_offset,
    }
}

pub fn retarget_from_bones(
    ctx: &RetargetContext,
    local_rotations: &BTreeMap<usize, Quaternion<f32>>,
    hips_offset: Vector3<f32>,
) -> SampledPose {
    let mut rotations = BTreeMap::new();

    for (&role, &bone_idx) in &ctx.mapping.by_role {
        let local_i = local_rotations
            .get(&bone_idx)
            .copied()
            .unwrap_or_else(Quaternion::one);

        let parent_rot = match ctx.skeleton.bones[bone_idx].parent {
            Some(parent_idx) => ctx.tpose_world[parent_idx],
            None => Quaternion::one(),
        };

        let t_i_inv = ctx.tpose_world[bone_idx].invert();
        let r_i = parent_rot * local_i * t_i_inv;

        rotations.insert(role, thyllore_rotation_to_role(&ctx.frame, r_i));
    }

    let m = Matrix3::from_cols(
        Vector3::from(ctx.frame.right),
        Vector3::from(ctx.frame.up),
        Vector3::from(ctx.frame.forward),
    );
    let hips_translation = m.transpose() * hips_offset / ctx.hips_height;

    SampledPose {
        rotations,
        hips_translation: hips_translation.into(),
        morph: BTreeMap::new(),
    }
}

pub fn pose_world_positions(
    ctx: &RetargetContext,
    retargeted: &RetargetedPose,
) -> Vec<Vector3<f32>> {
    let mut resolved = vec![None; ctx.skeleton.bones.len()];
    (0..resolved.len())
        .map(|bone_idx| resolve_world_transform(ctx, retargeted, bone_idx, &mut resolved).1)
        .collect()
}

fn resolve_world_transform(
    ctx: &RetargetContext,
    retargeted: &RetargetedPose,
    bone_idx: usize,
    resolved: &mut [Option<(Quaternion<f32>, Vector3<f32>)>],
) -> (Quaternion<f32>, Vector3<f32>) {
    if let Some(transform) = resolved[bone_idx] {
        return transform;
    }

    let bone = &ctx.skeleton.bones[bone_idx];
    let (parent_rotation, parent_position, bind_local_rotation, local_translation) = match bone
        .parent
    {
        Some(parent_idx) => {
            let (parent_rotation, parent_position) =
                resolve_world_transform(ctx, retargeted, parent_idx, resolved);
            let bind_parent = &ctx.skeleton.bones[parent_idx];
            let bind_parent_inverse = bind_parent.world_rotation.invert();
            (
                parent_rotation,
                parent_position,
                bind_parent_inverse * bone.world_rotation,
                bind_parent_inverse.rotate_vector(bone.world_position - bind_parent.world_position),
            )
        }
        None => (
            Quaternion::one(),
            Vector3::zero(),
            bone.world_rotation,
            bone.world_position,
        ),
    };

    let local_rotation = retargeted
        .local_rotations
        .get(&bone_idx)
        .copied()
        .unwrap_or(bind_local_rotation);
    let world_rotation = parent_rotation * local_rotation;
    let mut world_position = parent_position + parent_rotation.rotate_vector(local_translation);
    if ctx.mapping.by_role.get(&HumanoidRole::Hips) == Some(&bone_idx) {
        world_position += retargeted.hips_offset;
    }

    resolved[bone_idx] = Some((world_rotation, world_position));
    (world_rotation, world_position)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cgmath::{InnerSpace, One, Quaternion, Vector3};

    use crate::humanoid::components::humanoid_frame::HumanoidFrame;
    use crate::humanoid::components::mapping::HumanoidMapping;
    use crate::humanoid::components::rest_pose::RestPose;
    use crate::humanoid::components::role::HumanoidRole;
    use crate::motion::components::retarget_skeleton::{RetargetBone, RetargetSkeleton};
    use crate::motion::components::sampled_pose::SampledPose;

    use super::*;

    const HIPS: usize = 0;
    const SPINE: usize = 1;
    const HEAD: usize = 2;
    const LEFT_UPPER_ARM: usize = 3;
    const RIGHT_UPPER_ARM: usize = 4;
    const LEFT_LOWER_ARM: usize = 5;
    const RIGHT_LOWER_ARM: usize = 6;
    const LEFT_FOOT: usize = 7;
    const RIGHT_FOOT: usize = 8;

    fn default_frame() -> HumanoidFrame {
        HumanoidFrame {
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            forward: [0.0, 0.0, -1.0],
        }
    }

    fn bone(parent: Option<usize>, x: f32, y: f32, z: f32) -> RetargetBone {
        RetargetBone {
            parent,
            world_position: Vector3::new(x, y, z),
            world_rotation: Quaternion::one(),
        }
    }

    fn build_skeleton() -> RetargetSkeleton {
        RetargetSkeleton {
            bones: vec![
                bone(None, 0.0, 1.0, 0.0),
                bone(Some(HIPS), 0.0, 1.3, 0.0),
                bone(Some(SPINE), 0.0, 1.7, 0.0),
                bone(Some(SPINE), -0.2, 1.5, 0.0),
                bone(Some(SPINE), 0.2, 1.5, 0.0),
                bone(Some(LEFT_UPPER_ARM), -0.5, 1.5, 0.0),
                bone(Some(RIGHT_UPPER_ARM), 0.5, 1.5, 0.0),
                bone(Some(HIPS), -0.1, 0.0, 0.0),
                bone(Some(HIPS), 0.1, 0.0, 0.0),
            ],
        }
    }

    fn build_mapping() -> HumanoidMapping {
        let mut mapping = HumanoidMapping::default();
        mapping.by_role.insert(HumanoidRole::Hips, HIPS);
        mapping.by_role.insert(HumanoidRole::Spine, SPINE);
        mapping.by_role.insert(HumanoidRole::Head, HEAD);
        mapping
            .by_role
            .insert(HumanoidRole::LeftUpperArm, LEFT_UPPER_ARM);
        mapping
            .by_role
            .insert(HumanoidRole::RightUpperArm, RIGHT_UPPER_ARM);
        mapping
            .by_role
            .insert(HumanoidRole::LeftLowerArm, LEFT_LOWER_ARM);
        mapping
            .by_role
            .insert(HumanoidRole::RightLowerArm, RIGHT_LOWER_ARM);
        mapping.by_role.insert(HumanoidRole::LeftFoot, LEFT_FOOT);
        mapping.by_role.insert(HumanoidRole::RightFoot, RIGHT_FOOT);
        mapping
    }

    fn build_ctx(skeleton: &RetargetSkeleton, rest_pose: RestPose) -> RetargetContext {
        let mapping = build_mapping();
        let frame = default_frame();
        build_retarget_context(skeleton, &mapping, &frame, rest_pose).unwrap()
    }

    fn empty_pose() -> SampledPose {
        SampledPose {
            rotations: BTreeMap::new(),
            hips_translation: [0.0, 0.0, 0.0],
            morph: BTreeMap::new(),
        }
    }

    #[test]
    fn test_apose_zero_pose_right_lower_arm_position() {
        let mut skeleton = build_skeleton();
        let drop_sin = (std::f32::consts::PI / 4.0).sin();
        let drop_cos = (std::f32::consts::PI / 4.0).cos();
        skeleton.bones[RIGHT_UPPER_ARM].world_position = Vector3::new(0.2, 1.5, 0.0);
        skeleton.bones[RIGHT_LOWER_ARM].world_position =
            Vector3::new(0.2 + 0.3 * drop_cos, 1.5 - 0.3 * drop_sin, 0.0);

        let ctx = build_ctx(&skeleton, RestPose::APose);
        let pose = empty_pose();
        let retargeted = retarget_to_bones(&ctx, &pose);
        let positions = pose_world_positions(&ctx, &retargeted);

        let upper_pos = positions[RIGHT_UPPER_ARM];
        let lower_pos = positions[RIGHT_LOWER_ARM];
        let diff = lower_pos - upper_pos;
        let frame_right = Vector3::from(ctx.frame.right);
        let arm_length = diff.magnitude();

        assert!(
            (diff.normalize().dot(frame_right) - 1.0).abs() < 1e-4,
            "RightLowerArm should be along frame.right from RightUpperArm: diff={:?}, right={:?}",
            diff,
            frame_right
        );
        assert!(
            (arm_length - 0.3).abs() < 1e-4,
            "arm length should be ~0.3, got {}",
            arm_length
        );
    }

    #[test]
    fn test_tpose_right_upper_arm_z_plus_90() {
        let skeleton = build_skeleton();
        let ctx = build_ctx(&skeleton, RestPose::TPose);

        let mut pose = empty_pose();
        pose.rotations
            .insert(HumanoidRole::RightUpperArm, [0.0, 0.0, 90.0]);

        let retargeted = retarget_to_bones(&ctx, &pose);
        let positions = pose_world_positions(&ctx, &retargeted);

        let upper_pos = positions[RIGHT_UPPER_ARM];
        let lower_pos = positions[RIGHT_LOWER_ARM];
        let diff = lower_pos - upper_pos;
        let frame_up = Vector3::from(ctx.frame.up);

        assert!(
            (diff.normalize().dot(frame_up) - 1.0).abs() < 1e-4,
            "RightLowerArm should be above RightUpperArm along frame.up: diff={:?}, up={:?}",
            diff,
            frame_up
        );
    }

    #[test]
    fn test_spine_y_plus_90_and_right_upper_arm_z_plus_90() {
        let skeleton = build_skeleton();
        let ctx = build_ctx(&skeleton, RestPose::TPose);

        let mut pose = empty_pose();
        pose.rotations.insert(HumanoidRole::Spine, [0.0, 90.0, 0.0]);
        pose.rotations
            .insert(HumanoidRole::RightUpperArm, [0.0, 0.0, 90.0]);

        let retargeted = retarget_to_bones(&ctx, &pose);
        let positions = pose_world_positions(&ctx, &retargeted);

        let upper_pos = positions[RIGHT_UPPER_ARM];
        let lower_pos = positions[RIGHT_LOWER_ARM];
        let diff = lower_pos - upper_pos;
        let frame_up = Vector3::from(ctx.frame.up);

        assert!(
            (diff.normalize().dot(frame_up) - 1.0).abs() < 1e-4,
            "RightLowerArm should still be above RightUpperArm along frame.up: diff={:?}, up={:?}",
            diff,
            frame_up
        );

        let upper_rel = upper_pos - positions[SPINE];
        let upper_rest_rel =
            skeleton.bones[RIGHT_UPPER_ARM].world_position - skeleton.bones[SPINE].world_position;
        assert!(
            (upper_rel.magnitude() - upper_rest_rel.magnitude()).abs() < 1e-4,
            "RightUpperArm should keep its distance from Spine: {:?}",
            upper_rel
        );
        assert!(
            (upper_rel - upper_rest_rel).magnitude() > 0.1,
            "RightUpperArm should follow Spine rotation: {:?}",
            upper_rel
        );
    }

    #[test]
    fn test_hips_translation_negative_y() {
        let skeleton = build_skeleton();
        let ctx = build_ctx(&skeleton, RestPose::TPose);

        let mut pose = empty_pose();
        pose.hips_translation = [0.0, -0.5, 0.0];

        let retargeted = retarget_to_bones(&ctx, &pose);
        let positions = pose_world_positions(&ctx, &retargeted);

        let hips_rest = skeleton.bones[HIPS].world_position;
        let head_rest = skeleton.bones[HEAD].world_position;
        let frame_up = Vector3::from(ctx.frame.up);

        let hips_diff = positions[HIPS] - hips_rest;
        let expected_hips_offset = frame_up * (-0.5) * ctx.hips_height;
        assert!(
            (hips_diff - expected_hips_offset).magnitude() < 1e-4,
            "Hips should move by frame.up * -0.5 * hips_height: diff={:?}, expected={:?}",
            hips_diff,
            expected_hips_offset
        );

        let head_diff = positions[HEAD] - head_rest;
        assert!(
            (head_diff - expected_hips_offset).magnitude() < 1e-4,
            "Head should also move by the same offset: diff={:?}, expected={:?}",
            head_diff,
            expected_hips_offset
        );
    }

    #[test]
    fn retarget_from_bones_inverts_retarget_to_bones() {
        let skeleton = build_skeleton();
        let ctx = build_ctx(&skeleton, RestPose::TPose);

        let mut pose = empty_pose();
        pose.rotations
            .insert(HumanoidRole::Spine, [10.0, 20.0, -30.0]);
        pose.rotations
            .insert(HumanoidRole::LeftUpperArm, [0.0, 15.0, -70.0]);
        pose.rotations
            .insert(HumanoidRole::Head, [-20.0, 35.0, 5.0]);
        pose.hips_translation = [0.1, -0.05, 0.2];

        let retargeted = retarget_to_bones(&ctx, &pose);
        let recovered =
            retarget_from_bones(&ctx, &retargeted.local_rotations, retargeted.hips_offset);

        for role in ctx.mapping.by_role.keys() {
            let expected = pose.rotations.get(role).copied().unwrap_or([0.0; 3]);
            let actual = recovered.rotations[role];
            for axis in 0..3 {
                assert!(
                    (actual[axis] - expected[axis]).abs() < 1e-2,
                    "role {:?}: expected {:?}, got {:?}",
                    role,
                    expected,
                    actual
                );
            }
        }

        for axis in 0..3 {
            assert!(
                (recovered.hips_translation[axis] - pose.hips_translation[axis]).abs() < 1e-4,
                "hips: expected {:?}, got {:?}",
                pose.hips_translation,
                recovered.hips_translation
            );
        }
        assert!(recovered.morph.is_empty());
    }
}
