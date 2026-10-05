#![cfg(test)]
use std::fs;
use std::io::Write;

mod support;

use cgmath::{Matrix3, Matrix4, Quaternion, SquareMatrix};

use support::fbx_ascii::write_rig_fbx;
use support::rig_convention::rig_convention;
use support::rig_names::CONVENTIONS;
use support::rig_nodes::build_rig_nodes;

fn world_from_local(
    nodes: &[thyllore_importer_core::fbx::fbx::BoneNode],
    name: &str,
) -> Matrix4<f32> {
    let node = nodes.iter().find(|n| n.name == name).unwrap();
    let mut m = node.local_transform;
    let mut current_name = name;
    while let Some(ref parent_name) = nodes
        .iter()
        .find(|n| n.name == current_name)
        .unwrap()
        .parent
    {
        let parent = nodes
            .iter()
            .find(|n| n.name == parent_name.as_str())
            .unwrap();
        m = parent.local_transform * m;
        current_name = &parent.name;
    }
    m
}

fn quaternion_to_matrix(q: Quaternion<f32>) -> Matrix4<f32> {
    let m3: Matrix3<f32> = Matrix3::from(q);
    let mut m = Matrix4::identity();
    for r in 0..3 {
        for c in 0..3 {
            m[r][c] = m3[r][c];
        }
    }
    m
}

#[test]
fn test_skeleton_roundtrip_all_conventions() {
    for convention_id in &CONVENTIONS {
        let convention = rig_convention(convention_id);
        let fbx_text = write_rig_fbx(&convention);

        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!("skeleton_{}.fbx", convention_id));
        let mut file = fs::File::create(&temp_path).unwrap();
        file.write_all(fbx_text.as_bytes()).unwrap();

        let fbx_model =
            thyllore_importer_core::fbx::fbx::load_fbx_with_ufbx(temp_path.to_str().unwrap())
                .unwrap();

        fs::remove_file(&temp_path).ok();

        let nodes = build_rig_nodes(&convention);
        let canonical_bones = support::canonical_bones();
        let all_roles: Vec<&str> = canonical_bones.iter().map(|b| b.role).collect();

        for role in &all_roles {
            let bone_index = canonical_bones
                .iter()
                .position(|b| b.role == *role)
                .unwrap();
            let node = &nodes[bone_index];
            let name = &node.name;

            assert!(
                fbx_model.nodes.contains_key(name.as_str()),
                "convention {}: missing node \"{}\" (role {})",
                convention_id,
                name,
                role
            );
        }

        for node in &nodes {
            let world = world_from_local(
                &fbx_model.nodes.values().cloned().collect::<Vec<_>>(),
                &node.name,
            );

            let expected_pos_x =
                node.world_position.x as f32 * (convention.unit_scale_factor / 100.0) as f32;
            let expected_pos_y =
                node.world_position.y as f32 * (convention.unit_scale_factor / 100.0) as f32;
            let expected_pos_z =
                node.world_position.z as f32 * (convention.unit_scale_factor / 100.0) as f32;

            assert!(
                (world[3][0] - expected_pos_x).abs() < 1e-4,
                "convention {}: node \"{}\" translation x mismatch: got {}, expected {}",
                convention_id,
                node.name,
                world[3][0],
                expected_pos_x
            );
            assert!(
                (world[3][1] - expected_pos_y).abs() < 1e-4,
                "convention {}: node \"{}\" translation y mismatch: got {}, expected {}",
                convention_id,
                node.name,
                world[3][1],
                expected_pos_y
            );
            assert!(
                (world[3][2] - expected_pos_z).abs() < 1e-4,
                "convention {}: node \"{}\" translation z mismatch: got {}, expected {}",
                convention_id,
                node.name,
                world[3][2],
                expected_pos_z
            );

            let expected_q: Quaternion<f32> = Quaternion::new(
                node.world_rotation.s as f32,
                node.world_rotation.v.x as f32,
                node.world_rotation.v.y as f32,
                node.world_rotation.v.z as f32,
            );
            let expected_world_rot = quaternion_to_matrix(expected_q);

            for r in 0..3 {
                for c in 0..3 {
                    assert!(
                        (world[r][c] - expected_world_rot[r][c]).abs() < 1e-4,
                        "convention {}: node \"{}\" rotation [{},{}] mismatch: got {}, expected {}",
                        convention_id,
                        node.name,
                        r,
                        c,
                        world[r][c],
                        expected_world_rot[r][c]
                    );
                }
            }
        }
    }
}
