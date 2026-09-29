use std::collections::HashMap;
use std::fs;

mod support;

use cgmath::{Matrix4, Vector3};

use support::rig_names::bone_name;

fn nodes_to_bone_inputs(
    nodes: &HashMap<String, thyllore_importer_core::fbx::fbx::BoneNode>,
) -> Vec<thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput> {
    let mut node_list: Vec<_> = nodes.values().collect();
    node_list.sort_by(|a, b| a.name.cmp(&b.name));

    let name_to_index: HashMap<String, usize> = node_list
        .iter()
        .enumerate()
        .map(|(i, n)| (n.name.clone(), i))
        .collect();

    let n = node_list.len();
    let mut world_transforms: Vec<Option<Matrix4<f32>>> = vec![None; n];

    fn compute_world(
        idx: usize,
        node_list: &[&thyllore_importer_core::fbx::fbx::BoneNode],
        name_to_index: &HashMap<String, usize>,
        world_transforms: &mut Vec<Option<Matrix4<f32>>>,
    ) {
        if world_transforms[idx].is_some() {
            return;
        }
        let node = node_list[idx];
        if let Some(ref parent_name) = node.parent {
            let parent_idx = name_to_index[parent_name];
            compute_world(parent_idx, node_list, name_to_index, world_transforms);
        }
        let local = Matrix4::from(node.local_transform);
        let world = match &node.parent {
            Some(parent_name) => {
                let parent_idx = name_to_index[parent_name];
                world_transforms[parent_idx].unwrap() * local
            }
            None => local,
        };
        world_transforms[idx] = Some(world);
    }

    for i in 0..n {
        compute_world(i, &node_list, &name_to_index, &mut world_transforms);
    }

    let mut bones: Vec<thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput> =
        Vec::with_capacity(n);
    for (i, node) in node_list.iter().enumerate() {
        let world = world_transforms[i].unwrap();
        let pos = Vector3::new(world[3][0], world[3][1], world[3][2]);
        bones.push(
            thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput {
                name: node.name.clone(),
                parent: node
                    .parent
                    .as_ref()
                    .and_then(|p| name_to_index.get(p).copied()),
                rest_position: [pos.x, pos.y, pos.z],
            },
        );
    }

    bones
}

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
    let fbx_txt_path = format!("tests/data/rigs/{}.fbx.txt", convention_id);
    let fbx_text = fs::read_to_string(&fbx_txt_path).unwrap();

    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!("mapping_{}.fbx", convention_id));
    fs::write(&temp_path, fbx_text.as_bytes()).unwrap();

    let fbx_model =
        thyllore_importer_core::fbx::fbx::load_fbx_with_ufbx(temp_path.to_str().unwrap()).unwrap();
    fs::remove_file(&temp_path).ok();

    let bones = nodes_to_bone_inputs(&fbx_model.nodes);

    let (mapping, unresolved) = thyllore_avatar_core::humanoid::systems::infer::infer_mapping(
        &bones,
        &thyllore_avatar_core::humanoid::components::naming::HumanoidNamingRules::default(),
    );

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
