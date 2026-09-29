use std::collections::HashMap;
use std::fs;

use cgmath::{InnerSpace, Matrix3, Matrix4, Quaternion, Vector3};

pub fn load_fixture_bones(
    convention_id: &str,
) -> Vec<thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput> {
    load_fixture_rig(convention_id).0
}

pub fn load_fixture_rig(
    convention_id: &str,
) -> (
    Vec<thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput>,
    thyllore_avatar_core::motion::components::retarget_skeleton::RetargetSkeleton,
) {
    let fbx_txt_path = format!("tests/data/rigs/{}.fbx.txt", convention_id);
    let fbx_text = fs::read_to_string(&fbx_txt_path).unwrap();

    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!("mapping_{}.fbx", convention_id));
    fs::write(&temp_path, fbx_text.as_bytes()).unwrap();

    let fbx_model =
        thyllore_importer_core::fbx::fbx::load_fbx_with_ufbx(temp_path.to_str().unwrap()).unwrap();
    fs::remove_file(&temp_path).ok();

    nodes_to_fixture(&fbx_model.nodes)
}

fn nodes_to_fixture(
    nodes: &HashMap<String, thyllore_importer_core::fbx::fbx::BoneNode>,
) -> (
    Vec<thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput>,
    thyllore_avatar_core::motion::components::retarget_skeleton::RetargetSkeleton,
) {
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
    let mut retarget_bones: Vec<
        thyllore_avatar_core::motion::components::retarget_skeleton::RetargetBone,
    > = Vec::with_capacity(n);
    for (i, node) in node_list.iter().enumerate() {
        let world = world_transforms[i].unwrap();
        let pos = Vector3::new(world[3][0], world[3][1], world[3][2]);

        let col_x = Vector3::new(world[0][0], world[0][1], world[0][2]).normalize();
        let col_y = Vector3::new(world[1][0], world[1][1], world[1][2]).normalize();
        let col_z = Vector3::new(world[2][0], world[2][1], world[2][2]).normalize();
        let rotation_matrix = Matrix3::from_cols(col_x, col_y, col_z);
        let rot = Quaternion::from(rotation_matrix);

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
        retarget_bones.push(
            thyllore_avatar_core::motion::components::retarget_skeleton::RetargetBone {
                parent: node
                    .parent
                    .as_ref()
                    .and_then(|p| name_to_index.get(p).copied()),
                world_position: cgmath::Vector3::new(pos.x, pos.y, pos.z),
                world_rotation: rot,
            },
        );
    }

    let skeleton = thyllore_avatar_core::motion::components::retarget_skeleton::RetargetSkeleton {
        bones: retarget_bones,
    };

    (bones, skeleton)
}
