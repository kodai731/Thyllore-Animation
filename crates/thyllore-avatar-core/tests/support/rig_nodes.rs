#![cfg(test)]
use glam::{DQuat, DVec3};

use super::canonical_bones;
use super::rig_convention::{BoneAxisRule, ContainerNode, RigConvention};
use super::rig_names::bone_name;
use super::rig_positions::character_to_file_rotation;

#[derive(Clone, Debug)]
pub struct RigNode {
    pub name: String,
    pub parent: Option<usize>,
    pub role: Option<&'static str>,
    pub world_position: DVec3,
    pub world_rotation: DQuat,
}

pub fn build_rig_nodes(convention: &RigConvention) -> Vec<RigNode> {
    let bones = canonical_bones();
    let positions = super::rig_positions::file_positions(convention);

    let mut nodes: Vec<RigNode> = Vec::new();

    if let Some(container) = make_container_node(convention, &positions) {
        nodes.push(container);
    }

    let container_offset = nodes.len();

    for (i, bone) in bones.iter().enumerate() {
        let name = bone_name(convention.id, bone.role);
        let pos: DVec3 = [positions[i][0], positions[i][1], positions[i][2]].into();
        let child_bone_index = first_child_bone_index(&bones, i);
        let dir = bone_direction(&positions, i, child_bone_index);
        let world_rotation = node_world_rotation(convention, i, dir);
        nodes.push(RigNode {
            name,
            parent: match bone.parent {
                Some(p) => Some(p + container_offset),
                None if container_offset > 0 => Some(0),
                None => None,
            },
            role: Some(bone.role),
            world_position: pos,
            world_rotation,
        });
    }

    if convention.leaf_end_bones {
        let leaf_roles = ["Head", "RightHand", "LeftHand", "RightFoot", "LeftFoot"];
        for &leaf_role in &leaf_roles {
            let bone_index = bones.iter().position(|b| b.role == leaf_role).unwrap();
            let node_index = bone_index + container_offset;
            let parent_node = &nodes[node_index];
            let child_bone_index = first_child_bone_index(&bones, bone_index);
            let dir = bone_direction(&positions, bone_index, child_bone_index);
            let end_pos = parent_node.world_position
                + dir * (0.1 * convention.body_scale / (convention.unit_scale_factor / 100.0));
            nodes.push(RigNode {
                name: format!("{}_end", parent_node.name),
                parent: Some(node_index),
                role: None,
                world_position: end_pos,
                world_rotation: parent_node.world_rotation,
            });
        }
    }

    nodes
}

fn make_container_node(convention: &RigConvention, positions: &[[f64; 3]]) -> Option<RigNode> {
    let file_rotation = character_to_file_rotation(convention.character_to_file);
    match convention.container {
        ContainerNode::None => None,
        ContainerNode::Armature { rotation_x_degrees } => {
            let qx: DQuat =
                DQuat::from_axis_angle(DVec3::new(1.0, 0.0, 0.0), rotation_x_degrees.to_radians());
            let qc = DQuat::from_mat3(&file_rotation);
            let world_rotation = qc * qx;
            Some(RigNode {
                name: "Armature".to_string(),
                parent: None,
                role: None,
                world_position: DVec3::new(0.0, 0.0, 0.0),
                world_rotation,
            })
        }
        ContainerNode::BipedRoot {
            rotation_up_degrees,
        } => {
            let hips_pos = positions[0];
            let qy: DQuat =
                DQuat::from_axis_angle(DVec3::new(0.0, 1.0, 0.0), rotation_up_degrees.to_radians());
            let qc = DQuat::from_mat3(&file_rotation);
            let world_rotation = qc * qy;
            Some(RigNode {
                name: "Bip001".to_string(),
                parent: None,
                role: None,
                world_position: [hips_pos[0], hips_pos[1], hips_pos[2]].into(),
                world_rotation,
            })
        }
        ContainerNode::RootBone => Some(RigNode {
            name: "root".to_string(),
            parent: None,
            role: None,
            world_position: DVec3::new(0.0, 0.0, 0.0),
            world_rotation: DQuat::from_mat3(&file_rotation),
        }),
    }
}

pub fn first_child_bone_index(
    bones: &[super::CanonicalBone],
    parent_index: usize,
) -> Option<usize> {
    bones.iter().enumerate().find_map(|(i, bone)| {
        if bone.parent == Some(parent_index) {
            Some(i)
        } else {
            None
        }
    })
}

pub fn bone_direction(
    positions: &[[f64; 3]],
    bone_index: usize,
    child_bone_index: Option<usize>,
) -> DVec3 {
    match child_bone_index {
        Some(ci) => {
            let p: DVec3 = [
                positions[bone_index][0],
                positions[bone_index][1],
                positions[bone_index][2],
            ]
            .into();
            let c: DVec3 = [positions[ci][0], positions[ci][1], positions[ci][2]].into();
            (c - p).normalize()
        }
        None => {
            let bones = canonical_bones();
            if let Some(parent_index) = bones[bone_index].parent {
                let p: DVec3 = [
                    positions[bone_index][0],
                    positions[bone_index][1],
                    positions[bone_index][2],
                ]
                .into();
                let p_parent: DVec3 = [
                    positions[parent_index][0],
                    positions[parent_index][1],
                    positions[parent_index][2],
                ]
                .into();
                (p - p_parent).normalize()
            } else {
                DVec3::new(0.0, 1.0, 0.0)
            }
        }
    }
}

fn node_world_rotation(convention: &RigConvention, bone_index: usize, dir: DVec3) -> DQuat {
    match convention.bone_axis {
        BoneAxisRule::Along(a) => {
            let axis: DVec3 = [a[0], a[1], a[2]].into();
            let arc = shortest_arc(axis, dir);
            let roll_q: DQuat = DQuat::from_axis_angle(dir, convention.roll_degrees.to_radians());
            roll_q * arc
        }
        BoneAxisRule::WorldAligned => {
            let qc: DQuat =
                DQuat::from_mat3(&character_to_file_rotation(convention.character_to_file));
            qc
        }
        BoneAxisRule::Random { seed } => {
            let axis = bone_local_axis_for_random(seed, bone_index);
            let roll = roll_from_random(seed, bone_index);
            let arc = shortest_arc(axis, dir);
            let roll_q: DQuat = DQuat::from_axis_angle(dir, roll.to_radians());
            roll_q * arc
        }
    }
}

pub fn bone_local_axis(convention: &RigConvention, bone_index: usize) -> DVec3 {
    match convention.bone_axis {
        BoneAxisRule::Along(a) => [a[0], a[1], a[2]].into(),
        BoneAxisRule::WorldAligned => {
            let file_rotation = character_to_file_rotation(convention.character_to_file);
            let inv = file_rotation.transpose();
            let bones = canonical_bones();
            let child_bone_index = first_child_bone_index(&bones, bone_index);
            let positions = super::rig_positions::file_positions(convention);
            let dir = bone_direction(&positions, bone_index, child_bone_index);
            inv * dir
        }
        BoneAxisRule::Random { seed } => bone_local_axis_for_random(seed, bone_index),
    }
}

fn bone_local_axis_for_random(seed: u64, bone_index: usize) -> DVec3 {
    let mut state = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add((bone_index as u64).wrapping_mul(1442695040888963407))
        .wrapping_add(1);
    state = (state >> 32).wrapping_mul(2246822519).wrapping_shl(32) | (state & 0xFFFFFFFF);
    state = (state >> 32).wrapping_mul(2246822519).wrapping_shl(32) | (state & 0xFFFFFFFF);
    let choice = (state >> 32) as usize % 6;
    match choice {
        0 => DVec3::new(1.0, 0.0, 0.0),
        1 => DVec3::new(-1.0, 0.0, 0.0),
        2 => DVec3::new(0.0, 1.0, 0.0),
        3 => DVec3::new(0.0, -1.0, 0.0),
        4 => DVec3::new(0.0, 0.0, 1.0),
        _ => DVec3::new(0.0, 0.0, -1.0),
    }
}

fn roll_from_random(seed: u64, bone_index: usize) -> f64 {
    let mut state = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add((bone_index as u64).wrapping_mul(1442695040888963407))
        .wrapping_add(1);
    state = (state >> 32).wrapping_mul(2246822519).wrapping_shl(32) | (state & 0xFFFFFFFF);
    state = (state >> 32).wrapping_mul(2246822519).wrapping_shl(32) | (state & 0xFFFFFFFF);
    let choice = (state >> 32) as u32;
    (choice as f64 / 4294967295.0) * 360.0
}

fn shortest_arc(from: DVec3, to: DVec3) -> DQuat {
    let dot = from.dot(to);
    if dot < -1.0 + 1e-12 {
        let axis = perpendicular_to(from);
        DQuat::from_axis_angle(axis, std::f64::consts::PI)
    } else {
        let axis = from.cross(to);
        DQuat::from_xyzw(axis.x, axis.y, axis.z, 1.0 + dot).normalize()
    }
}

fn perpendicular_to(v: DVec3) -> DVec3 {
    let ax = v.x.abs();
    let ay = v.y.abs();
    let az = v.z.abs();
    if ax < ay && ax < az {
        DVec3::new(1.0, 0.0, 0.0).cross(v).normalize()
    } else if ay < az {
        DVec3::new(0.0, 1.0, 0.0).cross(v).normalize()
    } else {
        DVec3::new(0.0, 0.0, 1.0).cross(v).normalize()
    }
}
