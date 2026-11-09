#![cfg(test)]
use glam::{DMat3, DQuat, DVec3};

use super::rig_convention::RigConvention;
use super::{canonical_bones, CanonicalBone};

pub fn file_positions(convention: &RigConvention) -> Vec<[f64; 3]> {
    let bones = canonical_bones();
    let mut positions: Vec<[f64; 3]> = bones
        .iter()
        .enumerate()
        .map(|(i, _b)| {
            let mut p = character_space_position(&bones, i, convention);
            p[0] *= convention.body_scale / (convention.unit_scale_factor / 100.0);
            p[1] *= convention.body_scale / (convention.unit_scale_factor / 100.0);
            p[2] *= convention.body_scale / (convention.unit_scale_factor / 100.0);
            p
        })
        .collect();

    let file_rotation = character_to_file_rotation(convention.character_to_file);
    if file_rotation != DMat3::IDENTITY {
        for p in &mut positions {
            let v: DVec3 = [p[0], p[1], p[2]].into();
            let rotated = file_rotation * v;
            p[0] = rotated.x;
            p[1] = rotated.y;
            p[2] = rotated.z;
        }
    }

    positions
}

fn character_space_position(
    bones: &[CanonicalBone],
    index: usize,
    convention: &RigConvention,
) -> [f64; 3] {
    let role = bones[index].role;
    let mut p = [
        -bones[index].position[0] as f64,
        bones[index].position[1] as f64,
        bones[index].position[2] as f64,
    ];

    if convention.arm_drop_degrees != 0.0 {
        let drop = convention.arm_drop_degrees.to_radians();
        let cos_d = drop.cos();
        let sin_d = drop.sin();
        let upper_arm_role = match role {
            "RightLowerArm" | "RightHand" => Some("RightUpperArm"),
            "LeftLowerArm" | "LeftHand" => Some("LeftUpperArm"),
            _ => None,
        };
        if let Some(upper_arm_role) = upper_arm_role {
            let uai = bones.iter().position(|b| b.role == upper_arm_role).unwrap();
            let ux = -bones[uai].position[0] as f64;
            let uy = bones[uai].position[1] as f64;
            let ox = p[0] - ux;
            if ox != 0.0 {
                p[0] = ux + ox * cos_d;
                p[1] = uy - ox.abs() * sin_d;
            }
        }
    }

    p
}

pub fn character_to_file_rotation(spec: &[([f64; 3], f64)]) -> DMat3 {
    let mut m = DMat3::IDENTITY;
    for (axis, angle_deg) in spec {
        let axis_vec = DVec3::from(*axis);
        let norm = axis_vec.normalize();
        let angle = angle_deg.to_radians();
        let q: DQuat = DQuat::from_axis_angle(norm, angle);
        m = DMat3::from_quat(q) * m;
    }
    m
}
