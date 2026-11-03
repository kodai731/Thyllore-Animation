use cgmath::{InnerSpace, Vector3};

use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::role::HumanoidRole;

pub const T_POSE_TOLERANCE_DEG: f32 = 15.0;

#[derive(Clone, Copy)]
enum LimbDirection {
    Level,
    Down,
}

const T_POSE_LIMBS: [(HumanoidRole, HumanoidRole, LimbDirection); 4] = [
    (
        HumanoidRole::LeftUpperArm,
        HumanoidRole::LeftHand,
        LimbDirection::Level,
    ),
    (
        HumanoidRole::RightUpperArm,
        HumanoidRole::RightHand,
        LimbDirection::Level,
    ),
    (
        HumanoidRole::LeftUpperLeg,
        HumanoidRole::LeftFoot,
        LimbDirection::Down,
    ),
    (
        HumanoidRole::RightUpperLeg,
        HumanoidRole::RightFoot,
        LimbDirection::Down,
    ),
];

/// The limbs whose rest direction is not the VRM T-pose (arms level, legs straight down), one
/// warning each; `rest_heads` are the bones' rest heads in engine axes (Y up).
pub fn t_pose_warnings(
    mapping: &HumanoidMapping,
    rest_heads: &[Vector3<f32>],
    tolerance_deg: f32,
) -> Vec<String> {
    let head_of = |role| mapping.bone_of(role).and_then(|bone| rest_heads.get(bone));
    T_POSE_LIMBS
        .iter()
        .filter_map(|&(root_role, tip_role, expected)| {
            let direction = *head_of(tip_role)? - *head_of(root_role)?;
            let length = direction.magnitude();
            if length < 1e-6 {
                return None;
            }
            let unit_y = (direction.y / length).clamp(-1.0, 1.0);
            let deviation_deg = match expected {
                LimbDirection::Level => unit_y.asin().to_degrees().abs(),
                LimbDirection::Down => (-unit_y).acos().to_degrees(),
            };
            if deviation_deg <= tolerance_deg {
                return None;
            }
            let limb = match expected {
                LimbDirection::Level => "arm is",
                LimbDirection::Down => "leg is",
            };
            let reference = match expected {
                LimbDirection::Level => "level",
                LimbDirection::Down => "straight down",
            };
            Some(format!(
                "{}: {limb} {deviation_deg:.0}° from {reference} (VRM expects a T-pose)",
                root_role.unity_name()
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping() -> HumanoidMapping {
        let mut mapping = HumanoidMapping::default();
        for (index, role) in [
            HumanoidRole::LeftUpperArm,
            HumanoidRole::LeftHand,
            HumanoidRole::RightUpperArm,
            HumanoidRole::RightHand,
            HumanoidRole::LeftUpperLeg,
            HumanoidRole::LeftFoot,
        ]
        .into_iter()
        .enumerate()
        {
            mapping.by_role.insert(role, index);
        }
        mapping
    }

    fn heads(left_hand_y: f32, left_foot: Vector3<f32>) -> Vec<Vector3<f32>> {
        vec![
            Vector3::new(0.2, 1.4, 0.0),
            Vector3::new(0.7, left_hand_y, 0.0),
            Vector3::new(-0.2, 1.4, 0.0),
            Vector3::new(-0.7, 1.4, 0.0),
            Vector3::new(0.1, 0.9, 0.0),
            left_foot,
        ]
    }

    #[test]
    fn level_arms_and_straight_legs_pass() {
        let warnings = t_pose_warnings(
            &mapping(),
            &heads(1.4, Vector3::new(0.1, 0.1, 0.0)),
            T_POSE_TOLERANCE_DEG,
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn an_a_pose_arm_warns() {
        let warnings = t_pose_warnings(
            &mapping(),
            &heads(1.0, Vector3::new(0.1, 0.1, 0.0)),
            T_POSE_TOLERANCE_DEG,
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("LeftUpperArm"));
    }

    #[test]
    fn a_bent_leg_warns() {
        let warnings = t_pose_warnings(
            &mapping(),
            &heads(1.4, Vector3::new(0.5, 0.3, 0.0)),
            T_POSE_TOLERANCE_DEG,
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("LeftUpperLeg"));
    }

    #[test]
    fn unmapped_limbs_are_skipped() {
        let warnings = t_pose_warnings(
            &HumanoidMapping::default(),
            &heads(1.0, Vector3::new(0.5, 0.3, 0.0)),
            T_POSE_TOLERANCE_DEG,
        );
        assert!(warnings.is_empty());
    }
}
