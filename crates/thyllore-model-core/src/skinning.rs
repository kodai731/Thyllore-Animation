use cgmath::{Matrix4, Vector3, Vector4};

use crate::skeleton::Skeleton;
use crate::skin::SkinData;

pub struct SkinningDiagnostics {
    pub zero_weight_fallback_count: u32,
    pub near_origin_count: u32,
}

pub fn apply_skinning(
    skin_data: &SkinData,
    global_transforms: &[Matrix4<f32>],
    skeleton: &Skeleton,
    out_positions: &mut [Vector3<f32>],
    out_normals: &mut [Vector3<f32>],
) -> SkinningDiagnostics {
    let mut skin_matrices = Vec::with_capacity(skeleton.bone_count());

    for bone in &skeleton.bones {
        let global = global_transforms[bone.id as usize];
        let skin_matrix = global * bone.inverse_bind_pose;
        skin_matrices.push(skin_matrix);
    }

    let mut zero_weight_fallback_count: u32 = 0;
    let mut near_origin_count: u32 = 0;

    for i in 0..skin_data.base_positions.len() {
        let indices = &skin_data.bone_indices[i];
        let weights = &skin_data.bone_weights[i];

        let mut skinned_pos = Vector3::new(0.0, 0.0, 0.0);
        let mut skinned_normal = Vector3::new(0.0, 0.0, 0.0);
        let mut has_valid_weight = false;

        for j in 0..4 {
            let bone_idx = match j {
                0 => indices.x,
                1 => indices.y,
                2 => indices.z,
                3 => indices.w,
                _ => 0,
            } as usize;

            let weight = match j {
                0 => weights.x,
                1 => weights.y,
                2 => weights.z,
                3 => weights.w,
                _ => 0.0,
            };

            if weight > 0.0 && bone_idx < skin_matrices.len() {
                has_valid_weight = true;
                let m = &skin_matrices[bone_idx];

                let pos = skin_data.base_positions[i];
                let transformed = m * Vector4::new(pos.x, pos.y, pos.z, 1.0);
                skinned_pos += Vector3::new(transformed.x, transformed.y, transformed.z) * weight;

                if i < skin_data.base_normals.len() {
                    let normal = skin_data.base_normals[i];
                    let transformed_n = m * Vector4::new(normal.x, normal.y, normal.z, 0.0);
                    skinned_normal +=
                        Vector3::new(transformed_n.x, transformed_n.y, transformed_n.z) * weight;
                }
            }
        }

        if !has_valid_weight {
            zero_weight_fallback_count += 1;
            let fallback_bone = indices.x as usize;
            let pos = skin_data.base_positions[i];

            if fallback_bone < skin_matrices.len() {
                let m = &skin_matrices[fallback_bone];
                let transformed = m * Vector4::new(pos.x, pos.y, pos.z, 1.0);
                skinned_pos = Vector3::new(transformed.x, transformed.y, transformed.z);

                if i < skin_data.base_normals.len() {
                    let normal = skin_data.base_normals[i];
                    let transformed_n = m * Vector4::new(normal.x, normal.y, normal.z, 0.0);
                    skinned_normal =
                        Vector3::new(transformed_n.x, transformed_n.y, transformed_n.z);
                }
            } else {
                skinned_pos = pos;
                if i < skin_data.base_normals.len() {
                    skinned_normal = skin_data.base_normals[i];
                }
            }
        }

        if i < out_positions.len() {
            out_positions[i] = skinned_pos;
        }

        if skinned_pos.x.abs() < 0.001 && skinned_pos.y.abs() < 0.001 && skinned_pos.z.abs() < 0.001
        {
            near_origin_count += 1;
        }

        if i < out_normals.len() {
            let len = (skinned_normal.x * skinned_normal.x
                + skinned_normal.y * skinned_normal.y
                + skinned_normal.z * skinned_normal.z)
                .sqrt();
            if len > 0.0 {
                out_normals[i] = skinned_normal / len;
            }
        }
    }

    SkinningDiagnostics {
        zero_weight_fallback_count,
        near_origin_count,
    }
}
