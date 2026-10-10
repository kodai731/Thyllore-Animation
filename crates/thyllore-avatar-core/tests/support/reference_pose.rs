#![cfg(test)]
use cgmath::{Matrix3, One, Rad, Vector3};

use super::canonical_bones;

pub fn canonical_hips() -> [f32; 3] {
    canonical_bones()[0].position
}

pub fn canonical_hips_height() -> f32 {
    let hips = canonical_hips()[1];
    let foot_y = canonical_bones()
        .iter()
        .filter(|b| b.role == "LeftFoot" || b.role == "RightFoot")
        .map(|b| b.position[1])
        .fold(f32::INFINITY, f32::min);
    hips - foot_y
}

pub fn reference_world_positions(
    sampled: &thyllore_avatar_core::motion::components::sampled_pose::SampledPose,
) -> Vec<(
    thyllore_avatar_core::humanoid::components::role::HumanoidRole,
    [f32; 3],
)> {
    let bones = canonical_bones();
    let n = bones.len();

    let mut directions = vec![Matrix3::one(); n];
    let mut positions = vec![[0.0f32; 3]; n];

    let hips_translation: Vector3<f32> = [
        sampled.hips_translation[0],
        sampled.hips_translation[1],
        sampled.hips_translation[2],
    ]
    .into();
    let canonical_hips_pos: Vector3<f32> = [
        canonical_hips()[0],
        canonical_hips()[1],
        canonical_hips()[2],
    ]
    .into();
    let pos_root = canonical_hips_pos + hips_translation * canonical_hips_height();

    for i in 0..n {
        let bone = &bones[i];
        let role: thyllore_avatar_core::humanoid::components::role::HumanoidRole = match bone.role {
            "Hips" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::Hips,
            "Spine" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::Spine,
            "Chest" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::Chest,
            "Neck" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::Neck,
            "Head" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::Head,
            "RightShoulder" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightShoulder
            }
            "RightUpperArm" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightUpperArm
            }
            "RightLowerArm" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightLowerArm
            }
            "RightHand" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightHand
            }
            "LeftShoulder" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftShoulder
            }
            "LeftUpperArm" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftUpperArm
            }
            "LeftLowerArm" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftLowerArm
            }
            "LeftHand" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftHand,
            "RightUpperLeg" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightUpperLeg
            }
            "RightLowerLeg" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightLowerLeg
            }
            "RightFoot" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightFoot
            }
            "LeftUpperLeg" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftUpperLeg
            }
            "LeftLowerLeg" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftLowerLeg
            }
            "LeftFoot" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftFoot,
            _ => continue,
        };

        let euler = sampled.rotations.get(&role).copied().unwrap_or([0.0; 3]);
        let r_x = Matrix3::from_angle_x(Rad(euler[0].to_radians()));
        let r_y = Matrix3::from_angle_y(Rad(euler[1].to_radians()));
        let r_z = Matrix3::from_angle_z(Rad(euler[2].to_radians()));
        let r_i = r_y * r_x * r_z;

        let parent_dir = match bone.parent {
            Some(p) => directions[p],
            None => Matrix3::one(),
        };
        directions[i] = parent_dir * r_i;

        if i == 0 {
            positions[i] = [pos_root.x, pos_root.y, pos_root.z];
        } else {
            let p = bone.parent.unwrap();
            let parent_pos: Vector3<f32> =
                [positions[p][0], positions[p][1], positions[p][2]].into();
            let child_rest: Vector3<f32> =
                [bone.position[0], bone.position[1], bone.position[2]].into();
            let parent_rest: Vector3<f32> = [
                bones[p].position[0],
                bones[p].position[1],
                bones[p].position[2],
            ]
            .into();
            let offset = child_rest - parent_rest;
            let world_pos = parent_pos + parent_dir * offset;
            positions[i] = [world_pos.x, world_pos.y, world_pos.z];
        }
    }

    let mut result = Vec::new();
    for i in 0..n {
        let bone = &bones[i];
        let role: thyllore_avatar_core::humanoid::components::role::HumanoidRole = match bone.role {
            "Hips" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::Hips,
            "Spine" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::Spine,
            "Chest" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::Chest,
            "Neck" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::Neck,
            "Head" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::Head,
            "RightShoulder" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightShoulder
            }
            "RightUpperArm" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightUpperArm
            }
            "RightLowerArm" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightLowerArm
            }
            "RightHand" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightHand
            }
            "LeftShoulder" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftShoulder
            }
            "LeftUpperArm" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftUpperArm
            }
            "LeftLowerArm" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftLowerArm
            }
            "LeftHand" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftHand,
            "RightUpperLeg" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightUpperLeg
            }
            "RightLowerLeg" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightLowerLeg
            }
            "RightFoot" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightFoot
            }
            "LeftUpperLeg" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftUpperLeg
            }
            "LeftLowerLeg" => {
                thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftLowerLeg
            }
            "LeftFoot" => thyllore_avatar_core::humanoid::components::role::HumanoidRole::LeftFoot,
            _ => continue,
        };
        result.push((role, positions[i]));
    }
    result
}
