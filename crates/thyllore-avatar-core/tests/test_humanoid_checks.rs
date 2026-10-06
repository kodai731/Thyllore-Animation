#![cfg(test)]

use thyllore_avatar_core::humanoid::canonical::fixtures::{
    test_humanoid_bone_inputs, with_bone_in_two_roles, with_swapped_upper_arms, without_fingers,
};
use thyllore_avatar_core::humanoid::components::mapping_issues::MappingIssue;
use thyllore_avatar_core::humanoid::systems::validate::validate_mapping;

#[test]
fn test_valid_mapping_has_no_issues() {
    let (bones, mapping) = test_humanoid_bone_inputs();
    let issues = validate_mapping(&mapping, &bones);
    assert!(issues.is_empty(), "expected no issues, got: {:?}", issues);
}

#[test]
fn test_swapped_upper_arms_has_mirrored_roles_share_bone() {
    let (bones, mapping) = with_swapped_upper_arms();
    let issues = validate_mapping(&mapping, &bones);

    let mirrored: Vec<_> = issues
        .iter()
        .filter_map(|i| {
            if let MappingIssue::MirroredRolesShareBone { .. } = i {
                Some(i)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        mirrored.len(),
        1,
        "expected exactly 1 MirroredRolesShareBone, got: {:?}",
        issues
    );
}

#[test]
fn test_bone_in_two_roles_has_bone_in_two_roles() {
    let (bones, mapping) = with_bone_in_two_roles();
    let issues = validate_mapping(&mapping, &bones);

    let bone_in_two: Vec<_> = issues
        .iter()
        .filter_map(|i| {
            if let MappingIssue::BoneInTwoRoles { .. } = i {
                Some(i)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        bone_in_two.len(),
        1,
        "expected exactly 1 BoneInTwoRoles, got: {:?}",
        issues
    );

    let other: Vec<_> = issues
        .iter()
        .filter(|i| !matches!(i, MappingIssue::BoneInTwoRoles { .. }))
        .collect();
    assert!(
        other.is_empty(),
        "expected no other issues, got: {:?}",
        other
    );
}

#[test]
fn test_without_fingers_has_no_issues() {
    let (bones, mapping) = without_fingers();
    let issues = validate_mapping(&mapping, &bones);
    assert!(issues.is_empty(), "expected no issues, got: {:?}", issues);
}
