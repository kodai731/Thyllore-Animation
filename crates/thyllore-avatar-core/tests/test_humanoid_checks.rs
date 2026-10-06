#![cfg(test)]

use thyllore_avatar_core::humanoid::canonical::fixtures::{
    test_humanoid_bone_inputs, with_asymmetric_hand, with_bone_in_two_roles,
    with_long_left_upper_arm, with_spine_below_hips, with_swapped_upper_arms, without_fingers,
};
use thyllore_avatar_core::humanoid::components::geometry_warning::GeometryWarning;
use thyllore_avatar_core::humanoid::components::mapping_issues::MappingIssue;
use thyllore_avatar_core::humanoid::systems::character_frame::derive_character_frame;
use thyllore_avatar_core::humanoid::systems::geometry_checks::check_mapping_geometry;
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

#[test]
fn test_valid_geometry_has_no_warnings() {
    let (bones, mapping) = test_humanoid_bone_inputs();
    let frame = derive_character_frame(&mapping, &bones).unwrap();
    let warnings = check_mapping_geometry(&mapping, &bones, &frame);
    assert!(
        warnings.is_empty(),
        "expected no warnings, got: {:?}",
        warnings
    );
}

#[test]
fn test_spine_below_hips_has_not_ascending() {
    let (bones, mapping) = with_spine_below_hips();
    let frame = derive_character_frame(&mapping, &bones).unwrap();
    let warnings = check_mapping_geometry(&mapping, &bones, &frame);

    let not_ascending: Vec<_> = warnings
        .iter()
        .filter(|w| matches!(w, GeometryWarning::NotAscending { .. }))
        .collect();
    assert!(
        !not_ascending.is_empty(),
        "expected at least 1 NotAscending warning, got: {:?}",
        warnings
    );

    let other: Vec<_> = warnings
        .iter()
        .filter(|w| !matches!(w, GeometryWarning::NotAscending { .. }))
        .collect();
    assert!(
        other.is_empty(),
        "expected no other warnings, got: {:?}",
        other
    );
}

#[test]
fn test_long_left_upper_arm_has_length_ratio() {
    let (bones, mapping) = with_long_left_upper_arm();
    let frame = derive_character_frame(&mapping, &bones).unwrap();
    let warnings = check_mapping_geometry(&mapping, &bones, &frame);

    let length_ratio: Vec<_> = warnings
        .iter()
        .filter(|w| matches!(w, GeometryWarning::LengthRatio { .. }))
        .collect();
    assert!(
        !length_ratio.is_empty(),
        "expected at least 1 LengthRatio warning, got: {:?}",
        warnings
    );

    let other: Vec<_> = warnings
        .iter()
        .filter(|w| !matches!(w, GeometryWarning::LengthRatio { .. }))
        .collect();
    assert!(
        other.is_empty(),
        "expected no other warnings, got: {:?}",
        other
    );
}

#[test]
fn test_asymmetric_hand_has_asymmetric() {
    let (bones, mapping) = with_asymmetric_hand();
    let frame = derive_character_frame(&mapping, &bones).unwrap();
    let warnings = check_mapping_geometry(&mapping, &bones, &frame);

    let asymmetric: Vec<_> = warnings
        .iter()
        .filter(|w| matches!(w, GeometryWarning::Asymmetric { .. }))
        .collect();
    assert!(
        !asymmetric.is_empty(),
        "expected at least 1 Asymmetric warning, got: {:?}",
        warnings
    );

    let other: Vec<_> = warnings
        .iter()
        .filter(|w| !matches!(w, GeometryWarning::Asymmetric { .. }))
        .collect();
    assert!(
        other.is_empty(),
        "expected no other warnings, got: {:?}",
        other
    );
}

#[test]
fn test_bone_in_three_roles_has_bone_in_two_roles() {
    use std::collections::BTreeMap;

    let (bones, mut mapping) = test_humanoid_bone_inputs();
    let head_idx = *mapping
        .by_role
        .get(&thyllore_avatar_core::humanoid::components::role::HumanoidRole::Head)
        .unwrap();
    mapping.by_role.insert(
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::Jaw,
        head_idx,
    );
    mapping.by_role.insert(
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::Neck,
        head_idx,
    );
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
    assert!(
        !bone_in_two.is_empty(),
        "expected at least 1 BoneInTwoRoles for 3-role bone, got: {:?}",
        issues
    );
}
