use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::rest_pose::RestPose;
use crate::humanoid::components::role::HumanoidRole;
use crate::humanoid::components::skeleton_input::BoneInput;

use super::humanoid_frame::derive_humanoid_frame;

pub const T_POSE_ANGLE_THRESHOLD: f32 = 20.0;
pub const A_POSE_ANGLE_THRESHOLD: f32 = 60.0;

pub fn detect_rest_pose(mapping: &HumanoidMapping, bones: &[BoneInput]) -> RestPose {
    let frame = match derive_humanoid_frame(mapping, bones) {
        Some(f) => f,
        None => return RestPose::Unknown,
    };

    let Some(&upper_arm_idx) = mapping.by_role.get(&HumanoidRole::LeftUpperArm) else {
        return RestPose::Unknown;
    };
    let Some(&lower_arm_idx) = mapping.by_role.get(&HumanoidRole::LeftLowerArm) else {
        return RestPose::Unknown;
    };

    let upper_pos = frame.to_character(bones[upper_arm_idx].rest_position);
    let lower_pos = frame.to_character(bones[lower_arm_idx].rest_position);
    let angle = arm_angle_below_horizontal(upper_pos, lower_pos);

    if angle.abs() < T_POSE_ANGLE_THRESHOLD {
        RestPose::TPose
    } else if (T_POSE_ANGLE_THRESHOLD..A_POSE_ANGLE_THRESHOLD).contains(&angle) {
        RestPose::APose
    } else {
        RestPose::Unknown
    }
}

fn arm_angle_below_horizontal(upper_pos: [f32; 3], lower_pos: [f32; 3]) -> f32 {
    let dx = lower_pos[0] - upper_pos[0];
    let dy = lower_pos[1] - upper_pos[1];
    let dz = lower_pos[2] - upper_pos[2];
    let horizontal_length = (dx * dx + dz * dz).sqrt();
    (-dy).atan2(horizontal_length).to_degrees()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn bone(name: &str, parent: Option<usize>, pos: [f32; 3]) -> BoneInput {
        BoneInput {
            name: name.to_string(),
            parent,
            rest_position: pos,
        }
    }

    fn make_mapping(pairs: &[(HumanoidRole, usize)]) -> HumanoidMapping {
        let mut by_role = BTreeMap::new();
        for (role, idx) in pairs {
            by_role.insert(*role, *idx);
        }
        HumanoidMapping { by_role }
    }

    #[test]
    fn test_tpose_detection() {
        let bones: Vec<BoneInput> = vec![
            bone("Hips", None, [0.0, 1.0, 0.0]),
            bone("Spine", Some(0), [0.0, 2.0, 0.0]),
            bone("LeftShoulder", Some(1), [-0.3, 2.0, 0.0]),
            bone("LeftUpperArm", Some(2), [-0.5, 2.0, 0.0]),
            bone("Head", Some(1), [0.0, 2.5, 0.0]),
            bone("RightUpperArm", Some(2), [0.5, 2.0, 0.0]),
            bone("LeftLowerArm", Some(3), [-1.0, 2.0, 0.0]),
        ];
        let mapping = make_mapping(&[
            (HumanoidRole::Hips, 0),
            (HumanoidRole::Spine, 1),
            (HumanoidRole::LeftShoulder, 2),
            (HumanoidRole::LeftUpperArm, 3),
            (HumanoidRole::Head, 4),
            (HumanoidRole::RightUpperArm, 5),
            (HumanoidRole::LeftLowerArm, 6),
        ]);

        let pose = detect_rest_pose(&mapping, &bones);
        assert_eq!(pose, RestPose::TPose);
    }

    #[test]
    fn test_apose_detection() {
        let bones: Vec<BoneInput> = vec![
            bone("Hips", None, [0.0, 1.0, 0.0]),
            bone("Spine", Some(0), [0.0, 2.0, 0.0]),
            bone("LeftShoulder", Some(1), [-0.3, 2.0, 0.0]),
            bone("LeftUpperArm", Some(2), [-0.5, 2.0, 0.0]),
            bone("Head", Some(1), [0.0, 2.5, 0.0]),
            bone("RightUpperArm", Some(2), [0.5, 2.0, 0.0]),
            bone("LeftLowerArm", Some(3), [-0.8, 1.6, 0.0]),
        ];
        let mapping = make_mapping(&[
            (HumanoidRole::Hips, 0),
            (HumanoidRole::Spine, 1),
            (HumanoidRole::LeftShoulder, 2),
            (HumanoidRole::LeftUpperArm, 3),
            (HumanoidRole::Head, 4),
            (HumanoidRole::RightUpperArm, 5),
            (HumanoidRole::LeftLowerArm, 6),
        ]);

        let pose = detect_rest_pose(&mapping, &bones);
        assert_eq!(pose, RestPose::APose);
    }

    #[test]
    fn test_unknown_pose() {
        let bones: Vec<BoneInput> = vec![
            bone("Hips", None, [0.0, 1.0, 0.0]),
            bone("Spine", Some(0), [0.0, 2.0, 0.0]),
            bone("LeftShoulder", Some(1), [-0.3, 2.0, 0.0]),
            bone("LeftUpperArm", Some(2), [-0.5, 2.0, 0.0]),
            bone("Head", Some(1), [0.0, 2.5, 0.0]),
            bone("RightUpperArm", Some(2), [0.5, 2.0, 0.0]),
            bone("LeftLowerArm", Some(3), [-0.6, 1.0, 0.0]),
        ];
        let mapping = make_mapping(&[
            (HumanoidRole::Hips, 0),
            (HumanoidRole::Spine, 1),
            (HumanoidRole::LeftShoulder, 2),
            (HumanoidRole::LeftUpperArm, 3),
            (HumanoidRole::Head, 4),
            (HumanoidRole::RightUpperArm, 5),
            (HumanoidRole::LeftLowerArm, 6),
        ]);

        let pose = detect_rest_pose(&mapping, &bones);
        assert_eq!(pose, RestPose::Unknown);
    }

    #[test]
    fn test_missing_bone_returns_unknown() {
        let bones: Vec<BoneInput> = vec![bone("Hips", None, [0.0, 1.0, 0.0])];
        let mapping = make_mapping(&[(HumanoidRole::Hips, 0)]);

        let pose = detect_rest_pose(&mapping, &bones);
        assert_eq!(pose, RestPose::Unknown);
    }

    #[test]
    fn test_apose_detection_z_up() {
        fn y_to_z(p: [f32; 3]) -> [f32; 3] {
            [p[0], -p[2], p[1]]
        }
        let bones: Vec<BoneInput> = vec![
            bone("Hips", None, y_to_z([0.0, 1.0, 0.0])),
            bone("Spine", Some(0), y_to_z([0.0, 2.0, 0.0])),
            bone("LeftShoulder", Some(1), y_to_z([-0.3, 2.0, 0.0])),
            bone("LeftUpperArm", Some(2), y_to_z([-0.5, 2.0, 0.0])),
            bone("Head", Some(1), y_to_z([0.0, 2.5, 0.0])),
            bone("RightUpperArm", Some(2), y_to_z([0.5, 2.0, 0.0])),
            bone("LeftLowerArm", Some(3), y_to_z([-0.8, 1.6, 0.0])),
        ];
        let mapping = make_mapping(&[
            (HumanoidRole::Hips, 0),
            (HumanoidRole::Spine, 1),
            (HumanoidRole::LeftShoulder, 2),
            (HumanoidRole::LeftUpperArm, 3),
            (HumanoidRole::Head, 4),
            (HumanoidRole::RightUpperArm, 5),
            (HumanoidRole::LeftLowerArm, 6),
        ]);

        let pose = detect_rest_pose(&mapping, &bones);
        assert_eq!(pose, RestPose::APose);
    }
}
