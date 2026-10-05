#![cfg(test)]
use std::collections::HashMap;

use cgmath::{InnerSpace, One, Quaternion, Vector3};

use super::fbx_ascii::{cube_vertices, stick_polygon_indices, RigMesh};
use super::rig_convention::rig_convention;
use super::rig_nodes::RigNode;
use thyllore_avatar_core::expression::components::side::Side;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

#[derive(Clone, Debug)]
pub struct StickBone {
    pub role: HumanoidRole,
    pub parent: Option<usize>,
    pub position: [f64; 3],
    pub tip: StickTip,
}

#[derive(Clone, Debug)]
pub enum StickTip {
    Child(HumanoidRole),
    Leaf([f64; 3]),
}

fn mirror_tip(tip: &StickTip) -> StickTip {
    match tip {
        StickTip::Child(role) => StickTip::Child(role.mirrored()),
        StickTip::Leaf(offset) => StickTip::Leaf([-offset[0], offset[1], offset[2]]),
    }
}

#[rustfmt::skip]
const BONE_TABLE: &[(HumanoidRole, Option<HumanoidRole>, [f64; 3], StickTip)] = &[
    (HumanoidRole::Hips, None, [0.0, 0.95, 0.0], StickTip::Child(HumanoidRole::Spine)),
    (HumanoidRole::Spine, Some(HumanoidRole::Hips), [0.0, 1.05, 0.0], StickTip::Child(HumanoidRole::Chest)),
    (HumanoidRole::Chest, Some(HumanoidRole::Spine), [0.0, 1.20, 0.0], StickTip::Child(HumanoidRole::UpperChest)),
    (HumanoidRole::UpperChest, Some(HumanoidRole::Chest), [0.0, 1.32, 0.0], StickTip::Child(HumanoidRole::Neck)),
    (HumanoidRole::Neck, Some(HumanoidRole::UpperChest), [0.0, 1.45, 0.0], StickTip::Child(HumanoidRole::Head)),
    (HumanoidRole::Head, Some(HumanoidRole::Neck), [0.0, 1.55, 0.0], StickTip::Leaf([0.0, 0.18, 0.0])),
    (HumanoidRole::Jaw, Some(HumanoidRole::Head), [0.0, 1.53, 0.05], StickTip::Leaf([0.0, -0.04, 0.02])),
    (HumanoidRole::LeftEye, Some(HumanoidRole::Head), [0.03, 1.62, 0.07], StickTip::Leaf([0.0, 0.0, 0.03])),
    (HumanoidRole::LeftUpperLeg, Some(HumanoidRole::Hips), [0.09, 0.90, 0.0], StickTip::Child(HumanoidRole::LeftLowerLeg)),
    (HumanoidRole::LeftLowerLeg, Some(HumanoidRole::LeftUpperLeg), [0.09, 0.50, 0.0], StickTip::Child(HumanoidRole::LeftFoot)),
    (HumanoidRole::LeftFoot, Some(HumanoidRole::LeftLowerLeg), [0.09, 0.08, 0.0], StickTip::Child(HumanoidRole::LeftToes)),
    (HumanoidRole::LeftToes, Some(HumanoidRole::LeftFoot), [0.09, 0.02, 0.12], StickTip::Leaf([0.0, 0.0, 0.05])),
    (HumanoidRole::LeftShoulder, Some(HumanoidRole::UpperChest), [0.04, 1.40, 0.0], StickTip::Child(HumanoidRole::LeftUpperArm)),
    (HumanoidRole::LeftUpperArm, Some(HumanoidRole::LeftShoulder), [0.17, 1.40, 0.0], StickTip::Child(HumanoidRole::LeftLowerArm)),
    (HumanoidRole::LeftLowerArm, Some(HumanoidRole::LeftUpperArm), [0.45, 1.40, 0.0], StickTip::Child(HumanoidRole::LeftHand)),
    (HumanoidRole::LeftHand, Some(HumanoidRole::LeftLowerArm), [0.70, 1.40, 0.0], StickTip::Child(HumanoidRole::LeftMiddleProximal)),
    (HumanoidRole::LeftThumbProximal, Some(HumanoidRole::LeftHand), [0.73, 1.39, 0.03], StickTip::Child(HumanoidRole::LeftThumbIntermediate)),
    (HumanoidRole::LeftThumbIntermediate, Some(HumanoidRole::LeftThumbProximal), [0.76, 1.39, 0.05], StickTip::Child(HumanoidRole::LeftThumbDistal)),
    (HumanoidRole::LeftThumbDistal, Some(HumanoidRole::LeftThumbIntermediate), [0.78, 1.39, 0.06], StickTip::Leaf([0.02, 0.0, 0.0])),
    (HumanoidRole::LeftIndexProximal, Some(HumanoidRole::LeftHand), [0.78, 1.40, 0.025], StickTip::Child(HumanoidRole::LeftIndexIntermediate)),
    (HumanoidRole::LeftIndexIntermediate, Some(HumanoidRole::LeftIndexProximal), [0.82, 1.40, 0.025], StickTip::Child(HumanoidRole::LeftIndexDistal)),
    (HumanoidRole::LeftIndexDistal, Some(HumanoidRole::LeftIndexIntermediate), [0.85, 1.40, 0.025], StickTip::Leaf([0.02, 0.0, 0.0])),
    (HumanoidRole::LeftMiddleProximal, Some(HumanoidRole::LeftHand), [0.785, 1.40, 0.0], StickTip::Child(HumanoidRole::LeftMiddleIntermediate)),
    (HumanoidRole::LeftMiddleIntermediate, Some(HumanoidRole::LeftMiddleProximal), [0.83, 1.40, 0.0], StickTip::Child(HumanoidRole::LeftMiddleDistal)),
    (HumanoidRole::LeftMiddleDistal, Some(HumanoidRole::LeftMiddleIntermediate), [0.865, 1.40, 0.0], StickTip::Leaf([0.02, 0.0, 0.0])),
    (HumanoidRole::LeftRingProximal, Some(HumanoidRole::LeftHand), [0.78, 1.40, -0.02], StickTip::Child(HumanoidRole::LeftRingIntermediate)),
    (HumanoidRole::LeftRingIntermediate, Some(HumanoidRole::LeftRingProximal), [0.82, 1.40, -0.02], StickTip::Child(HumanoidRole::LeftRingDistal)),
    (HumanoidRole::LeftRingDistal, Some(HumanoidRole::LeftRingIntermediate), [0.85, 1.40, -0.02], StickTip::Leaf([0.02, 0.0, 0.0])),
    (HumanoidRole::LeftLittleProximal, Some(HumanoidRole::LeftHand), [0.77, 1.40, -0.04], StickTip::Child(HumanoidRole::LeftLittleIntermediate)),
    (HumanoidRole::LeftLittleIntermediate, Some(HumanoidRole::LeftLittleProximal), [0.80, 1.40, -0.04], StickTip::Child(HumanoidRole::LeftLittleDistal)),
    (HumanoidRole::LeftLittleDistal, Some(HumanoidRole::LeftLittleIntermediate), [0.825, 1.40, -0.04], StickTip::Leaf([0.02, 0.0, 0.0])),
];

pub fn test_humanoid_bones() -> Vec<StickBone> {
    let mut bones: Vec<StickBone> = Vec::with_capacity(55);
    let mut role_to_idx: HashMap<HumanoidRole, usize> = HashMap::with_capacity(55);

    for (role, parent_role, position, tip) in BONE_TABLE {
        let idx = bones.len();
        let parent = parent_role.map(|pr| *role_to_idx.get(&pr).unwrap());
        bones.push(StickBone {
            role: *role,
            parent,
            position: *position,
            tip: tip.clone(),
        });
        role_to_idx.insert(*role, idx);
    }

    for (role, parent_role, position, tip) in BONE_TABLE {
        if role.side() != Some(Side::Left) {
            continue;
        }
        let new_role = role.mirrored();
        let new_pos: [f64; 3] = [-position[0], position[1], position[2]];
        let new_tip = mirror_tip(tip);
        let new_parent = parent_role.map(|pr| {
            let mirrored_parent_role = pr.mirrored();
            *role_to_idx.get(&mirrored_parent_role).unwrap()
        });
        let idx = bones.len();
        bones.push(StickBone {
            role: new_role,
            parent: new_parent,
            position: new_pos,
            tip: new_tip,
        });
        role_to_idx.insert(new_role, idx);
    }

    bones
}

pub fn test_humanoid_nodes(bones: &[StickBone]) -> Vec<RigNode> {
    bones
        .iter()
        .enumerate()
        .map(|(i, bone)| RigNode {
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

pub fn build_stick_mesh(bones: &[StickBone]) -> RigMesh {
    let mut vertices: Vec<[f64; 3]> = Vec::new();
    let mut polygon_vertex_index: Vec<i32> = Vec::new();
    let mut cluster_vertex_indices: Vec<Vec<i32>> = Vec::new();

    for (bi, bone) in bones.iter().enumerate() {
        let center = Vector3::new(bone.position[0], bone.position[1], bone.position[2]);
        let tip_pos = match &bone.tip {
            StickTip::Child(child_role) => {
                let child_idx = bones
                    .iter()
                    .enumerate()
                    .find(|(_, b)| b.role == *child_role)
                    .map(|(i, _)| i)
                    .unwrap();
                let child = &bones[child_idx];
                Vector3::new(child.position[0], child.position[1], child.position[2])
            }
            StickTip::Leaf(offset) => center + Vector3::new(offset[0], offset[1], offset[2]),
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
    let bones = test_humanoid_bones();
    let nodes = test_humanoid_nodes(&bones);
    let mesh = build_stick_mesh(&bones);
    super::fbx_ascii::write_rig_fbx_with_mesh(&rig_convention("vrm_normalized"), &nodes, &mesh)
}
