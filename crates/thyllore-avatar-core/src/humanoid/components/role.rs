use crate::expression::components::side::Side;

include!(concat!(env!("OUT_DIR"), "/humanoid_rig.rs"));

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn test_paired_part_declares_both_sides() {
        assert_eq!(HumanoidRole::LeftUpperArm.side(), Some(Side::Left));
        assert_eq!(HumanoidRole::RightUpperArm.side(), Some(Side::Right));
        assert_eq!(
            HumanoidRole::LeftUpperArm.name_patterns(),
            HumanoidRole::RightUpperArm.name_patterns()
        );
    }

    #[test]
    fn test_centre_part_has_no_side() {
        assert_eq!(HumanoidRole::Hips.side(), None);
        assert_eq!(HumanoidRole::Hips.unity_name(), "Hips");
    }

    #[test]
    fn test_unity_names_are_unique() {
        let names: BTreeSet<&str> = HumanoidRole::ALL
            .iter()
            .map(|role| role.unity_name())
            .collect();
        assert_eq!(names.len(), HumanoidRole::ALL.len());
    }

    #[test]
    fn test_required_roles_and_chains_come_from_all() {
        let chained = HUMANOID_CHAINS.iter().flat_map(|chain| chain.iter());
        for role in REQUIRED.iter().chain(chained) {
            assert!(HumanoidRole::ALL.contains(role));
        }
    }

    #[test]
    fn test_allows_translation_only_hips() {
        assert!(HumanoidRole::Hips.allows_translation());
        for role in HumanoidRole::ALL
            .iter()
            .filter(|r| **r != HumanoidRole::Hips)
        {
            assert!(
                !role.allows_translation(),
                "{role:?} should not allow translation"
            );
        }
    }

    #[test]
    fn test_from_unity_name_roundtrip() {
        for role in HumanoidRole::ALL {
            assert_eq!(
                HumanoidRole::from_unity_name(role.unity_name()),
                Some(role),
                "from_unity_name({:?}) should return Some({role:?})",
                role.unity_name()
            );
        }
    }

    #[test]
    fn test_index_matches_all_position() {
        for (i, &role) in HumanoidRole::ALL.iter().enumerate() {
            assert_eq!(role.index(), i, "{role:?}.index() should be {i}");
            assert_eq!(HumanoidRole::ALL[role.index()], role);
        }
    }
}
