#![cfg(test)]
mod support;

use support::fixture_bones::load_fixture_bones;
use support::rig_names::bone_name;

fn role_to_string(
    role: thyllore_avatar_core::humanoid::components::role::HumanoidRole,
) -> &'static str {
    match role {
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::Hips => "Hips",
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::Spine => "Spine",
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::Chest => "Chest",
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::UpperChest => "UpperChest",
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::Neck => "Neck",
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::Head => "Head",
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftShoulder => {
            "LeftShoulder"
        }
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightShoulder => {
            "RightShoulder"
        }
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftUpperArm => {
            "LeftUpperArm"
        }
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightUpperArm => {
            "RightUpperArm"
        }
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftLowerArm => {
            "LeftLowerArm"
        }
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightLowerArm => {
            "RightLowerArm"
        }
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftHand => "LeftHand",
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightHand => "RightHand",
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftUpperLeg => {
            "LeftUpperLeg"
        }
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightUpperLeg => {
            "RightUpperLeg"
        }
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftLowerLeg => {
            "LeftLowerLeg"
        }
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightLowerLeg => {
            "RightLowerLeg"
        }
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftFoot => "LeftFoot",
        thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightFoot => "RightFoot",
        _ => panic!("unexpected role"),
    }
}

fn test_convention(convention_id: &str) {
    let bones = load_fixture_bones(convention_id);

    let (mapping, unresolved) =
        thyllore_avatar_core::humanoid::systems::name_match::infer_mapping(&bones);

    let unresolved_required: Vec<_> = unresolved
        .iter()
        .filter(|u| thyllore_avatar_core::humanoid::components::role::REQUIRED.contains(&u.role))
        .collect();
    assert!(
        unresolved_required.is_empty(),
        "convention {}: unresolved required roles: {:?}",
        convention_id,
        unresolved_required
    );

    for role in thyllore_avatar_core::humanoid::components::role::REQUIRED {
        let bone_index = mapping.by_role.get(&role).unwrap_or_else(|| {
            panic!(
                "convention {}: required role {:?} not resolved",
                convention_id, role
            )
        });
        let bone_name_actual = bones[*bone_index].name.as_str();
        let expected_name = bone_name(convention_id, role_to_string(role));
        assert_eq!(
            bone_name_actual,
            expected_name.as_str(),
            "convention {}: role {:?} resolved to \"{}\" but expected \"{}\"",
            convention_id,
            role,
            bone_name_actual,
            expected_name
        );
    }
}

#[test]
fn test_blender() {
    test_convention("blender");
}

#[test]
fn test_maya() {
    test_convention("maya");
}

#[test]
fn test_max_biped() {
    test_convention("max_biped");
}

#[test]
fn test_mixamo() {
    test_convention("mixamo");
}

#[test]
fn test_unreal() {
    test_convention("unreal");
}

#[test]
fn test_vrm_normalized() {
    test_convention("vrm_normalized");
}

#[test]
fn test_blender_apose() {
    test_convention("blender_apose");
}

#[test]
fn test_adversarial() {
    test_convention("adversarial");
}
