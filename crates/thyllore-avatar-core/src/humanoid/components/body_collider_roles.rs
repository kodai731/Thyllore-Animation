use std::collections::BTreeSet;

use super::mapping::HumanoidMapping;
use super::role::BODY_COLLIDER;

pub fn body_collider_bone_indices(mapping: &HumanoidMapping) -> BTreeSet<usize> {
    BODY_COLLIDER
        .iter()
        .filter_map(|&role| mapping.by_role.get(&role).copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humanoid::components::role::HumanoidRole;

    #[test]
    fn test_body_collider_bone_indices_filters_thin_and_unassigned() {
        let mut mapping = HumanoidMapping::default();
        mapping.by_role.insert(HumanoidRole::Hips, 0);
        mapping.by_role.insert(HumanoidRole::LeftThumbProximal, 1);
        mapping.by_role.insert(HumanoidRole::LeftUpperArm, 2);

        let indices = body_collider_bone_indices(&mapping);

        assert_eq!(indices, BTreeSet::from([0usize, 2]));
    }

    #[test]
    fn test_body_collider_roles_exclude_fingers_shoulders_and_toes() {
        for role in BODY_COLLIDER {
            assert!(HumanoidRole::ALL.contains(&role));
        }
        for excluded in [
            HumanoidRole::LeftShoulder,
            HumanoidRole::LeftToes,
            HumanoidRole::LeftIndexProximal,
            HumanoidRole::Jaw,
        ] {
            assert!(!BODY_COLLIDER.contains(&excluded), "{excluded:?}");
        }
    }
}
