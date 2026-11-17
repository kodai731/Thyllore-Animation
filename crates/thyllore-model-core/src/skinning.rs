use glam::{Mat4, Vec3, Vec4};

use crate::mesh::Vertex;
use crate::skeleton::Skeleton;
use crate::skin::SkinData;
use crate::skin::INFLUENCES_PER_VERTEX;

pub struct SkinningDiagnostics {
    pub zero_weight_fallback_count: u32,
    pub near_origin_count: u32,
}

fn skin_vertex(skin_data: &SkinData, skin_matrices: &[Mat4], i: usize) -> (Vec3, Vec3, bool) {
    let indices = &skin_data.bone_indices[i];
    let weights = &skin_data.bone_weights[i];

    let mut skinned_pos = Vec3::new(0.0, 0.0, 0.0);
    let mut skinned_normal = Vec3::new(0.0, 0.0, 0.0);
    let mut has_valid_weight = false;

    for j in 0..INFLUENCES_PER_VERTEX {
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
            let transformed = m * Vec4::new(pos.x, pos.y, pos.z, 1.0);
            skinned_pos += Vec3::new(transformed.x, transformed.y, transformed.z) * weight;

            if i < skin_data.base_normals.len() {
                let normal = skin_data.base_normals[i];
                let transformed_n = m * Vec4::new(normal.x, normal.y, normal.z, 0.0);
                skinned_normal +=
                    Vec3::new(transformed_n.x, transformed_n.y, transformed_n.z) * weight;
            }
        }
    }

    if !has_valid_weight {
        let fallback_bone = indices.x as usize;
        let pos = skin_data.base_positions[i];

        if fallback_bone < skin_matrices.len() {
            let m = &skin_matrices[fallback_bone];
            let transformed = m * Vec4::new(pos.x, pos.y, pos.z, 1.0);
            skinned_pos = Vec3::new(transformed.x, transformed.y, transformed.z);

            if i < skin_data.base_normals.len() {
                let normal = skin_data.base_normals[i];
                let transformed_n = m * Vec4::new(normal.x, normal.y, normal.z, 0.0);
                skinned_normal = Vec3::new(transformed_n.x, transformed_n.y, transformed_n.z);
            }
        } else {
            skinned_pos = pos;
            if i < skin_data.base_normals.len() {
                skinned_normal = skin_data.base_normals[i];
            }
        }
    }

    (skinned_pos, skinned_normal, has_valid_weight)
}

fn skin_matrices(skeleton: &Skeleton, global_transforms: &[Mat4]) -> Vec<Mat4> {
    let mut skin_matrices = Vec::with_capacity(skeleton.bone_count());
    for bone in &skeleton.bones {
        let global = global_transforms[bone.id as usize];
        let skin_matrix = global * bone.inverse_bind_pose;
        skin_matrices.push(skin_matrix);
    }
    skin_matrices
}

fn skin_each_vertex(
    skin_data: &SkinData,
    global_transforms: &[Mat4],
    skeleton: &Skeleton,
    mut write: impl FnMut(usize, Vec3, Option<Vec3>),
) -> SkinningDiagnostics {
    let skin_matrices = skin_matrices(skeleton, global_transforms);

    let mut zero_weight_fallback_count: u32 = 0;
    let mut near_origin_count: u32 = 0;

    for i in 0..skin_data.base_positions.len() {
        let (skinned_pos, skinned_normal, had_valid_weight) =
            skin_vertex(skin_data, &skin_matrices, i);

        if !had_valid_weight {
            zero_weight_fallback_count += 1;
        }

        if skinned_pos.x.abs() < 0.001 && skinned_pos.y.abs() < 0.001 && skinned_pos.z.abs() < 0.001
        {
            near_origin_count += 1;
        }

        let unit_normal = {
            let len = (skinned_normal.x * skinned_normal.x
                + skinned_normal.y * skinned_normal.y
                + skinned_normal.z * skinned_normal.z)
                .sqrt();
            if len > 0.0 {
                Some(skinned_normal / len)
            } else {
                None
            }
        };

        write(i, skinned_pos, unit_normal);
    }

    SkinningDiagnostics {
        zero_weight_fallback_count,
        near_origin_count,
    }
}

pub fn apply_skinning(
    skin_data: &SkinData,
    global_transforms: &[Mat4],
    skeleton: &Skeleton,
    out_positions: &mut [Vec3],
    out_normals: &mut [Vec3],
) -> SkinningDiagnostics {
    skin_each_vertex(skin_data, global_transforms, skeleton, |i, pos, normal| {
        if i < out_positions.len() {
            out_positions[i] = pos;
        }
        if let Some(n) = normal {
            if i < out_normals.len() {
                out_normals[i] = n;
            }
        }
    })
}

pub fn apply_skinning_to_vertices(
    skin_data: &SkinData,
    global_transforms: &[Mat4],
    skeleton: &Skeleton,
    vertices: &mut [Vertex],
) -> SkinningDiagnostics {
    skin_each_vertex(skin_data, global_transforms, skeleton, |i, pos, normal| {
        if i < vertices.len() {
            vertices[i].pos = pos;
            match normal {
                Some(n) => vertices[i].normal = n,
                None => vertices[i].normal = Vec3::Y,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use glam::UVec4;

    use super::*;

    #[test]
    fn apply_skinning_to_vertices_matches_apply_skinning() {
        let mut skeleton = Skeleton::new("test");
        let root = skeleton.add_bone("root", None);
        skeleton.add_bone("child", Some(root));

        let global_transforms = vec![
            Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Mat4::from_rotation_z(37.0_f32.to_radians()),
        ];

        let skin_data = SkinData {
            skeleton_id: 0,
            bone_indices: vec![
                UVec4::new(0, 0, 0, 0),
                UVec4::new(0, 1, 0, 0),
                UVec4::new(1, 0, 0, 0),
                UVec4::new(1, 0, 0, 0),
                UVec4::new(0, 0, 0, 0),
            ],
            bone_weights: vec![
                Vec4::new(1.0, 0.0, 0.0, 0.0),
                Vec4::new(0.3, 0.7, 0.0, 0.0),
                Vec4::new(0.6, 0.4, 0.0, 0.0),
                Vec4::new(0.0, 0.0, 0.0, 0.0),
                Vec4::new(1.0, 0.0, 0.0, 0.0),
            ],
            base_positions: vec![
                Vec3::new(0.2, 1.0, -0.4),
                Vec3::new(-0.7, 2.0, 0.3),
                Vec3::new(0.5, 3.0, 0.9),
                Vec3::new(0.1, 4.0, -0.6),
                Vec3::new(0.0, 0.0, 0.0),
            ],
            base_normals: vec![
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(0.6, 0.8, 0.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 0.0),
            ],
        };

        let vertex_count = skin_data.base_positions.len();
        let mut out_positions = vec![Vec3::new(0.0, 0.0, 0.0); vertex_count];
        let mut out_normals = vec![Vec3::new(0.0, 1.0, 0.0); vertex_count];
        let diag1 = apply_skinning(
            &skin_data,
            &global_transforms,
            &skeleton,
            &mut out_positions,
            &mut out_normals,
        );

        let mut vertices = vec![Vertex::default(); skin_data.base_positions.len()];
        for v in &mut vertices {
            v.normal = Vec3::Y;
        }

        let diag2 =
            apply_skinning_to_vertices(&skin_data, &global_transforms, &skeleton, &mut vertices);

        assert_eq!(
            diag1.zero_weight_fallback_count,
            diag2.zero_weight_fallback_count
        );
        assert_eq!(diag1.near_origin_count, diag2.near_origin_count);

        for (i, vertex) in vertices.iter().enumerate() {
            assert_eq!(
                bits_of(out_positions[i]),
                bits_of(vertex.pos),
                "position at vertex {i}"
            );
            assert_eq!(
                bits_of(out_normals[i]),
                bits_of(vertex.normal),
                "normal at vertex {i}"
            );
        }
    }

    fn bits_of(v: Vec3) -> [u32; 3] {
        [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]
    }
}
