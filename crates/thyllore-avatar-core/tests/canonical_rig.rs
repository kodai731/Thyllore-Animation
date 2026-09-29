mod support;

use support::canonical_bones;

#[test]
fn test_count_is_19() {
    let bones = canonical_bones();
    assert_eq!(bones.len(), 19, "expected 19 bones");
}

#[test]
fn test_left_right_x_symmetry() {
    let bones = canonical_bones();

    let pairs: [(&str, &str); 7] = [
        ("RightShoulder", "LeftShoulder"),
        ("RightUpperArm", "LeftUpperArm"),
        ("RightLowerArm", "LeftLowerArm"),
        ("RightHand", "LeftHand"),
        ("RightUpperLeg", "LeftUpperLeg"),
        ("RightLowerLeg", "LeftLowerLeg"),
        ("RightFoot", "LeftFoot"),
    ];

    for (right_role, left_role) in &pairs {
        let right = bones.iter().find(|b| b.role == *right_role).unwrap();
        let left = bones.iter().find(|b| b.role == *left_role).unwrap();

        assert_eq!(
            right.position[1], left.position[1],
            "y mismatch: {} vs {}",
            right_role, left_role
        );
        assert_eq!(
            right.position[2], left.position[2],
            "z mismatch: {} vs {}",
            right_role, left_role
        );
        assert_eq!(
            right.position[0], -left.position[0],
            "x not sign-flipped: {} ({}) vs {} ({})",
            right_role, right.position[0], left_role, left.position[0]
        );
    }
}

#[test]
fn test_both_arms_horizontal() {
    let bones = canonical_bones();

    let right_arm_roles: &[&str] = &[
        "RightShoulder",
        "RightUpperArm",
        "RightLowerArm",
        "RightHand",
    ];
    let left_arm_roles: &[&str] = &["LeftShoulder", "LeftUpperArm", "LeftLowerArm", "LeftHand"];

    for roles in [right_arm_roles, left_arm_roles] {
        let first_y = bones.iter().find(|b| b.role == roles[0]).unwrap().position[1];
        for &role in &roles[1..] {
            let bone = bones.iter().find(|b| b.role == role).unwrap();
            assert_eq!(
                bone.position[1], first_y,
                "arm not horizontal: {} y={} vs first y={}",
                role, bone.position[1], first_y
            );
        }
    }
}

#[test]
fn test_parent_index_less_than_self() {
    let bones = canonical_bones();

    for (i, bone) in bones.iter().enumerate() {
        if let Some(parent) = bone.parent {
            assert!(
                parent < i,
                "bone {} ({}) has parent index {} >= self index {}",
                i,
                bone.role,
                parent,
                i
            );
        }
    }
}
