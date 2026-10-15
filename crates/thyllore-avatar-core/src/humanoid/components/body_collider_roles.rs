use std::collections::BTreeSet;

use super::mapping::HumanoidMapping;
use super::role::HumanoidRole;

pub const BODY_COLLIDER_ROLES: &[HumanoidRole] = &[
    HumanoidRole::Hips,
    HumanoidRole::Spine,
    HumanoidRole::Chest,
    HumanoidRole::UpperChest,
    HumanoidRole::Neck,
    HumanoidRole::Head,
    HumanoidRole::LeftUpperArm,
    HumanoidRole::RightUpperArm,
    HumanoidRole::LeftLowerArm,
    HumanoidRole::RightLowerArm,
    HumanoidRole::LeftHand,
    HumanoidRole::RightHand,
    HumanoidRole::LeftUpperLeg,
    HumanoidRole::RightUpperLeg,
    HumanoidRole::LeftLowerLeg,
    HumanoidRole::RightLowerLeg,
    HumanoidRole::LeftFoot,
    HumanoidRole::RightFoot,
];

pub fn body_collider_bone_indices(mapping: &HumanoidMapping) -> BTreeSet<usize> {
    BODY_COLLIDER_ROLES
        .iter()
        .filter_map(|&role| mapping.by_role.get(&role).copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_body_collider_bone_indices_filters_thin_and_unassigned() {
        let mut mapping = HumanoidMapping::default();
        mapping.by_role.insert(HumanoidRole::Hips, 0);
        mapping.by_role.insert(HumanoidRole::LeftThumbProximal, 1);
        mapping.by_role.insert(HumanoidRole::LeftUpperArm, 2);

        let indices = body_collider_bone_indices(&mapping);

        assert_eq!(indices, BTreeSet::from([0usize, 2]));
    }
}
