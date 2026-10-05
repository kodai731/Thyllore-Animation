#![cfg(test)]
use cgmath::{InnerSpace, One, Quaternion, Vector3};

use super::fbx_ascii::{cube_vertices, stick_polygon_indices, RigMesh};
use super::rig_convention::rig_convention;
use super::rig_nodes::RigNode;
pub use thyllore_avatar_core::humanoid::canonical::skeleton::{
    skeleton, CanonicalBone, CanonicalTip,
};
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

pub fn test_humanoid_nodes(bones: &[CanonicalBone]) -> Vec<RigNode> {
    bones
        .iter()
        .enumerate()
        .map(|(_i, bone)| RigNode {
            name: bone.role.unity_name().to_string(),
            parent: bone.parent,
            role: Some(bone.role.unity_name()),
            world_position: Vector3::new(bone.position[0], bone.position[1], bone.position[2]),
            world_rotation: Quaternion::one(),
        })
        .collect()
}

fn is_finger_or_eye(name: &str) -> bool {
    ["Thumb", "Index", "Middle", "Ring", "Little", "Eye", "Jaw"]
        .iter()
        .any(|s| name.contains(s))
}

pub fn build_stick_mesh(bones: &[CanonicalBone]) -> RigMesh {
    let mut vertices: Vec<[f64; 3]> = Vec::new();
    let mut polygon_vertex_index: Vec<i32> = Vec::new();
    let mut cluster_vertex_indices: Vec<Vec<i32>> = Vec::new();

    for (bi, bone) in bones.iter().enumerate() {
        let center = Vector3::new(bone.position[0], bone.position[1], bone.position[2]);
        let tip_pos = match &bone.tip {
            CanonicalTip::Child(child_role) => {
                let child_idx = bones
                    .iter()
                    .enumerate()
                    .find(|(_, b)| b.role == *child_role)
                    .map(|(i, _)| i)
                    .unwrap();
                let child = &bones[child_idx];
                Vector3::new(child.position[0], child.position[1], child.position[2])
            }
            CanonicalTip::Leaf(offset) => center + Vector3::new(offset[0], offset[1], offset[2]),
        };
        let dir = (tip_pos - center).normalize();
        let length = (tip_pos - center).magnitude();
        let half = if is_finger_or_eye(bone.role.unity_name()) {
            0.008
        } else {
            0.02
        };

        let verts = cube_vertices(center, dir, length, half);
        let base = vertices.len() as i32;
        for v in &verts {
            vertices.push(*v);
        }
        polygon_vertex_index.extend_from_slice(&stick_polygon_indices(base));
        cluster_vertex_indices.push((base..base + 8).collect());

        if bone.role == HumanoidRole::Head {
            let head_top = cube_vertices(center, Vector3::new(0.0, 1.0, 0.0), 0.2, 0.09);
            let head_base = vertices.len() as i32;
            for v in &head_top {
                vertices.push(*v);
            }
            polygon_vertex_index.extend_from_slice(&stick_polygon_indices(head_base));

            let nose_center = center + Vector3::new(0.0, 0.09, 0.0);
            let nose = cube_vertices(nose_center, Vector3::new(0.0, 0.0, 1.0), 0.11, 0.015);
            let nose_base = vertices.len() as i32;
            for v in &nose {
                vertices.push(*v);
            }
            polygon_vertex_index.extend_from_slice(&stick_polygon_indices(nose_base));

            cluster_vertex_indices[bi] = (base..base + 24).collect();
        }
    }

    RigMesh {
        vertices,
        polygon_vertex_index,
        cluster_vertex_indices,
        diffuse_color: Some([1.0, 1.0, 1.0]),
    }
}

pub fn write_test_humanoid_fbx() -> String {
    let bones = skeleton();
    let nodes = test_humanoid_nodes(&bones);
    let mesh = build_stick_mesh(&bones);
    super::fbx_ascii::write_rig_fbx_with_mesh(&rig_convention("vrm_normalized"), &nodes, &mesh)
}
