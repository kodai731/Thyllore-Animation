#![cfg(test)]
use cgmath::{Euler, InnerSpace, One, Quaternion, Vector3};

use super::fbx_ascii::{cube_vertices, stick_polygon_indices, ClothMaterial, RigMesh};
use super::rig_convention::rig_convention;
use super::rig_nodes::RigNode;
pub use thyllore_avatar_core::humanoid::canonical::fixtures::{
    extra_bones, ExtraBone, ExtraParent,
};
pub use thyllore_avatar_core::humanoid::canonical::skeleton::{
    skeleton, CanonicalBone, CanonicalTip,
};
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

const EXTRA_BONE_HALF_WIDTH: f64 = 0.012;

pub fn test_humanoid_nodes(bones: &[CanonicalBone], extras: &[ExtraBone]) -> Vec<RigNode> {
    let mut nodes: Vec<RigNode> = bones
        .iter()
        .enumerate()
        .map(|(_i, bone)| RigNode {
            name: bone.role.unity_name().to_string(),
            parent: bone.parent,
            role: Some(bone.role.unity_name()),
            world_position: Vector3::new(bone.position[0], bone.position[1], bone.position[2]),
            world_rotation: Quaternion::one(),
        })
        .collect();

    let bones_len = bones.len();
    for extra in extras {
        let parent_index = match extra.parent {
            ExtraParent::Role(r) => bones.iter().position(|b| b.role == r).unwrap(),
            ExtraParent::Row(i) => bones_len + i,
        };
        let parent_world_rotation = nodes[parent_index].world_rotation;
        let rest_rot = Quaternion::from(Euler::new(
            cgmath::Deg(extra.rest_euler_degrees[0]),
            cgmath::Deg(extra.rest_euler_degrees[1]),
            cgmath::Deg(extra.rest_euler_degrees[2]),
        ));
        let world_rotation = parent_world_rotation * rest_rot;

        nodes.push(RigNode {
            name: extra.name.clone(),
            parent: Some(parent_index),
            role: None,
            world_position: Vector3::new(extra.position[0], extra.position[1], extra.position[2]),
            world_rotation,
        });
    }

    nodes
}

fn is_finger_or_eye(name: &str) -> bool {
    ["Thumb", "Index", "Middle", "Ring", "Little", "Eye", "Jaw"]
        .iter()
        .any(|s| name.contains(s))
}

pub fn build_stick_mesh(bones: &[CanonicalBone], extras: &[ExtraBone]) -> RigMesh {
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

    let role_stick_polygon_count = polygon_vertex_index.iter().filter(|&&v| v < 0).count();

    for extra in extras.iter() {
        let center = Vector3::new(extra.position[0], extra.position[1], extra.position[2]);
        let tip = Vector3::new(extra.tip[0], extra.tip[1], extra.tip[2]);
        let dir = tip.normalize();
        let length = tip.magnitude();

        let verts = cube_vertices(center, dir, length, EXTRA_BONE_HALF_WIDTH);
        let base = vertices.len() as i32;
        for v in &verts {
            vertices.push(*v);
        }
        polygon_vertex_index.extend_from_slice(&stick_polygon_indices(base));
        cluster_vertex_indices.push((base..base + 8).collect());
    }

    let total_nodes = bones.len() + extras.len();
    let cluster_nodes: Vec<usize> = (0..total_nodes).collect();

    RigMesh {
        vertices,
        polygon_vertex_index,
        cluster_vertex_indices,
        cluster_nodes,
        diffuse_color: Some([1.0, 1.0, 1.0]),
        cloth: Some(ClothMaterial {
            color: [0.55, 0.55, 0.55],
            first_polygon: role_stick_polygon_count,
        }),
    }
}

pub fn write_test_humanoid_fbx() -> String {
    let bones = skeleton();
    let extras = extra_bones();
    let nodes = test_humanoid_nodes(&bones, &extras);
    let mesh = build_stick_mesh(&bones, &extras);
    super::fbx_ascii::write_rig_fbx_with_mesh(&rig_convention("vrm_normalized"), &nodes, &mesh)
}
