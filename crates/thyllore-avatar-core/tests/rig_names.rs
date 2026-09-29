mod support;

use support::canonical_bones;
use support::rig_names::{bone_name, CONVENTIONS};

#[test]
fn test_all_conventions_have_non_empty_names_for_all_roles() {
    let roles = canonical_bones();
    for &convention in &CONVENTIONS {
        for bone in &roles {
            let name = bone_name(convention, bone.role);
            assert!(
                !name.is_empty(),
                "convention={}, role={}: name is empty",
                convention,
                bone.role
            );
        }
    }
}

#[test]
fn test_no_duplicate_names_within_convention() {
    let roles = canonical_bones();
    for &convention in &CONVENTIONS {
        let mut names: Vec<String> = Vec::new();
        for bone in &roles {
            let name = bone_name(convention, bone.role);
            assert!(
                !names.contains(&name),
                "convention={}: duplicate name {:?} for role {} (first seen at a different role)",
                convention,
                name,
                bone.role
            );
            names.push(name);
        }
        assert_eq!(
            names.len(),
            19,
            "convention={}: expected 19 unique names, got {}",
            convention,
            names.len()
        );
    }
}

#[test]
fn test_maya_right_upper_arm() {
    assert_eq!(bone_name("maya", "RightUpperArm"), "RightArm");
}

#[test]
fn test_blender_left_hand() {
    assert_eq!(bone_name("blender", "LeftHand"), "Hand.L");
}

#[test]
fn test_max_biped_left_foot() {
    assert_eq!(bone_name("max_biped", "LeftFoot"), "Bip001 L Foot");
}

#[test]
fn test_mixamo_hips() {
    assert_eq!(bone_name("mixamo", "Hips"), "mixamorig:Hips");
}

#[test]
fn test_unreal_spine() {
    assert_eq!(bone_name("unreal", "Spine"), "spine_01");
}

#[test]
fn test_vrm_normalized_head() {
    assert_eq!(bone_name("vrm_normalized", "Head"), "J_Bip_C_Head");
}

#[test]
fn test_blender_apose_neck() {
    assert_eq!(bone_name("blender_apose", "Neck"), "Neck");
}

#[test]
fn test_adversarial_chest() {
    assert_eq!(bone_name("adversarial", "Chest"), "mixamorig:Spine1");
}
