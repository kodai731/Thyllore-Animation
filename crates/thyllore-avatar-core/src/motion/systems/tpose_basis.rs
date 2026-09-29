use cgmath::{InnerSpace, Quaternion, Vector3};

use crate::humanoid::components::character_frame::CharacterFrame;
use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::rest_pose::RestPose;
use crate::humanoid::components::role::HumanoidRole;
use crate::motion::components::retarget_skeleton::RetargetSkeleton;

pub fn compute_tpose_world_rotations(
    skeleton: &RetargetSkeleton,
    mapping: &HumanoidMapping,
    frame: &CharacterFrame,
    rest_pose: RestPose,
) -> Vec<Quaternion<f32>> {
    let mut rotations: Vec<_> = skeleton.bones.iter().map(|b| b.world_rotation).collect();

    if rest_pose == RestPose::Unknown {
        return rotations;
    }

    let right = Vector3::from(frame.right);
    apply_arm_correction(
        &mut rotations,
        skeleton,
        mapping,
        HumanoidRole::LeftUpperArm,
        HumanoidRole::LeftLowerArm,
        -right,
    );
    apply_arm_correction(
        &mut rotations,
        skeleton,
        mapping,
        HumanoidRole::RightUpperArm,
        HumanoidRole::RightLowerArm,
        right,
    );

    rotations
}

fn apply_arm_correction(
    rotations: &mut [Quaternion<f32>],
    skeleton: &RetargetSkeleton,
    mapping: &HumanoidMapping,
    upper_role: HumanoidRole,
    lower_role: HumanoidRole,
    target_direction: Vector3<f32>,
) {
    let (Some(&upper_idx), Some(&lower_idx)) = (
        mapping.by_role.get(&upper_role),
        mapping.by_role.get(&lower_role),
    ) else {
        return;
    };

    let bind_direction =
        skeleton.bones[lower_idx].world_position - skeleton.bones[upper_idx].world_position;
    if bind_direction.magnitude2() < 1e-12 {
        return;
    }

    let correction = Quaternion::from_arc(
        bind_direction.normalize(),
        target_direction.normalize(),
        None,
    );

    let mut visited = vec![false; skeleton.bones.len()];
    let mut stack = vec![upper_idx];
    while let Some(bone_idx) = stack.pop() {
        if visited[bone_idx] {
            continue;
        }
        visited[bone_idx] = true;
        rotations[bone_idx] = correction * rotations[bone_idx];
        for (child_idx, bone) in skeleton.bones.iter().enumerate() {
            if bone.parent == Some(bone_idx) && !visited[child_idx] {
                stack.push(child_idx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{InnerSpace, One, Quaternion, Rotation, Vector3};

    use crate::humanoid::components::character_frame::CharacterFrame;
    use crate::humanoid::components::mapping::HumanoidMapping;
    use crate::humanoid::components::rest_pose::RestPose;
    use crate::humanoid::components::role::HumanoidRole;
    use crate::motion::components::retarget_skeleton::{RetargetBone, RetargetSkeleton};

    use super::compute_tpose_world_rotations;

    const HIPS: usize = 0;
    const SPINE: usize = 1;
    const HEAD: usize = 2;
    const LEFT_UPPER_ARM: usize = 3;
    const RIGHT_UPPER_ARM: usize = 4;
    const LEFT_LOWER_ARM: usize = 5;
    const RIGHT_LOWER_ARM: usize = 6;

    fn default_frame() -> CharacterFrame {
        CharacterFrame {
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            forward: [0.0, 0.0, -1.0],
        }
    }

    fn bone(parent: Option<usize>, x: f32, y: f32) -> RetargetBone {
        RetargetBone {
            parent,
            world_position: Vector3::new(x, y, 0.0),
            world_rotation: Quaternion::one(),
        }
    }

    fn build_skeleton(arm_drop_radians: f32) -> RetargetSkeleton {
        let (drop_sin, drop_cos) = arm_drop_radians.sin_cos();
        RetargetSkeleton {
            bones: vec![
                bone(None, 0.0, 1.0),
                bone(Some(HIPS), 0.0, 1.3),
                bone(Some(SPINE), 0.0, 1.7),
                bone(Some(SPINE), -0.2, 1.5),
                bone(Some(SPINE), 0.2, 1.5),
                bone(
                    Some(LEFT_UPPER_ARM),
                    -0.2 - 0.3 * drop_cos,
                    1.5 - 0.3 * drop_sin,
                ),
                bone(
                    Some(RIGHT_UPPER_ARM),
                    0.2 + 0.3 * drop_cos,
                    1.5 - 0.3 * drop_sin,
                ),
            ],
        }
    }

    fn build_mapping(left_lower_arm: usize) -> HumanoidMapping {
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
            .insert(HumanoidRole::LeftLowerArm, left_lower_arm);
        mapping
            .by_role
            .insert(HumanoidRole::RightLowerArm, RIGHT_LOWER_ARM);
        mapping
    }

    fn corrected_arm_direction(
        skeleton: &RetargetSkeleton,
        rotations: &[Quaternion<f32>],
        upper_idx: usize,
        lower_idx: usize,
    ) -> Vector3<f32> {
        let bind_direction =
            skeleton.bones[lower_idx].world_position - skeleton.bones[upper_idx].world_position;
        rotations[lower_idx]
            .rotate_vector(bind_direction)
            .normalize()
    }

    fn assert_rotation_close(actual: Quaternion<f32>, expected: Quaternion<f32>) {
        assert!(
            (actual - expected).magnitude() < 1e-5,
            "{:?} != {:?}",
            actual,
            expected
        );
    }

    #[test]
    fn apose_arms_are_rotated_onto_character_right() {
        let skeleton = build_skeleton(std::f32::consts::FRAC_PI_4);
        let mapping = build_mapping(LEFT_LOWER_ARM);
        let frame = default_frame();
        let right = Vector3::from(frame.right);

        let rotations = compute_tpose_world_rotations(&skeleton, &mapping, &frame, RestPose::APose);

        let left = corrected_arm_direction(&skeleton, &rotations, LEFT_UPPER_ARM, LEFT_LOWER_ARM);
        let right_arm =
            corrected_arm_direction(&skeleton, &rotations, RIGHT_UPPER_ARM, RIGHT_LOWER_ARM);
        assert!(left.dot(-right) > 0.9999, "left arm: {:?}", left);
        assert!(right_arm.dot(right) > 0.9999, "right arm: {:?}", right_arm);
        for idx in [HIPS, SPINE, HEAD] {
            assert_rotation_close(rotations[idx], Quaternion::one());
        }
    }

    #[test]
    fn tpose_input_keeps_every_rotation() {
        let skeleton = build_skeleton(0.0);
        let mapping = build_mapping(LEFT_LOWER_ARM);

        let rotations =
            compute_tpose_world_rotations(&skeleton, &mapping, &default_frame(), RestPose::TPose);

        for (rotation, bone) in rotations.iter().zip(&skeleton.bones) {
            assert_rotation_close(*rotation, bone.world_rotation);
        }
    }

    #[test]
    fn unknown_rest_pose_returns_bind_rotations() {
        let skeleton = build_skeleton(std::f32::consts::FRAC_PI_4);
        let mapping = build_mapping(LEFT_LOWER_ARM);

        let rotations =
            compute_tpose_world_rotations(&skeleton, &mapping, &default_frame(), RestPose::Unknown);

        for (rotation, bone) in rotations.iter().zip(&skeleton.bones) {
            assert_eq!(*rotation, bone.world_rotation);
        }
    }

    #[test]
    fn unmapped_intermediate_bone_passes_correction_to_lower_arm() {
        let mut skeleton = build_skeleton(std::f32::consts::FRAC_PI_4);
        let upper = skeleton.bones[LEFT_UPPER_ARM].world_position;
        let lower = skeleton.bones[LEFT_LOWER_ARM].world_position;
        let intermediate_idx = skeleton.bones.len();
        skeleton.bones.push(RetargetBone {
            parent: Some(LEFT_UPPER_ARM),
            world_position: (upper + lower) * 0.5,
            world_rotation: Quaternion::one(),
        });
        skeleton.bones[LEFT_LOWER_ARM].parent = Some(intermediate_idx);
        let mapping = build_mapping(LEFT_LOWER_ARM);
        let frame = default_frame();

        let rotations = compute_tpose_world_rotations(&skeleton, &mapping, &frame, RestPose::APose);

        let left = corrected_arm_direction(&skeleton, &rotations, LEFT_UPPER_ARM, LEFT_LOWER_ARM);
        assert!(
            left.dot(-Vector3::from(frame.right)) > 0.9999,
            "left arm: {:?}",
            left
        );
        assert_rotation_close(rotations[intermediate_idx], rotations[LEFT_UPPER_ARM]);
    }
}
