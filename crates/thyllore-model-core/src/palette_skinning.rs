use glam::{Mat4, Vec3};

use crate::mesh::VertexGeometry;
use crate::skeleton::Skeleton;
use crate::skin::{SkinData, INFLUENCES_PER_VERTEX};
use crate::skinning::skin_matrices;

/// One vertex of the influence stream: the palette entries that move it and their weights.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct SkinInfluence {
    pub palette_indices: [u32; INFLUENCES_PER_VERTEX],
    pub weights: [f32; INFLUENCES_PER_VERTEX],
}

/// The skin matrix of every bone in bone order, then one identity entry for vertices no bone moves.
pub fn build_skin_palette(skeleton: &Skeleton, global_transforms: &[Mat4]) -> Vec<Mat4> {
    let mut palette = skin_matrices(skeleton, global_transforms);
    palette.push(Mat4::IDENTITY);
    palette
}

pub fn skin_palette_len(bone_count: usize) -> usize {
    bone_count + 1
}

pub fn pack_skin_influences(skin_data: &SkinData, bone_count: usize) -> Vec<SkinInfluence> {
    skin_data
        .bone_indices
        .iter()
        .zip(&skin_data.bone_weights)
        .map(|(indices, weights)| {
            pack_vertex_influence(indices.to_array(), weights.to_array(), bone_count)
        })
        .collect()
}

fn pack_vertex_influence(
    bone_indices: [u32; INFLUENCES_PER_VERTEX],
    bone_weights: [f32; INFLUENCES_PER_VERTEX],
    bone_count: usize,
) -> SkinInfluence {
    let names_bone = |bone_index: u32| (bone_index as usize) < bone_count;

    let mut influence = SkinInfluence::default();
    let mut has_weighted_bone = false;
    for slot in 0..INFLUENCES_PER_VERTEX {
        if bone_weights[slot] > 0.0 && names_bone(bone_indices[slot]) {
            influence.palette_indices[slot] = bone_indices[slot];
            influence.weights[slot] = bone_weights[slot];
            has_weighted_bone = true;
        }
    }
    if has_weighted_bone {
        return influence;
    }

    let rigid_entry = if names_bone(bone_indices[0]) {
        bone_indices[0]
    } else {
        bone_count as u32
    };
    SkinInfluence {
        palette_indices: [rigid_entry, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    }
}

pub fn collect_rest_geometry(skin_data: &SkinData) -> Vec<VertexGeometry> {
    skin_data
        .base_positions
        .iter()
        .enumerate()
        .map(|(vertex, &pos)| VertexGeometry {
            pos,
            normal: skin_data
                .base_normals
                .get(vertex)
                .copied()
                .unwrap_or(Vec3::ZERO),
        })
        .collect()
}

/// Linear blend skinning: each vertex is the weighted sum of its rest geometry moved by its palette entries.
pub fn skin_geometry(
    rest: &[VertexGeometry],
    influences: &[SkinInfluence],
    palette: &[Mat4],
    skinned: &mut [VertexGeometry],
) {
    for ((target, rest_vertex), influence) in skinned.iter_mut().zip(rest).zip(influences) {
        let mut pos = Vec3::ZERO;
        let mut normal = Vec3::ZERO;
        for slot in 0..INFLUENCES_PER_VERTEX {
            let skin_matrix = palette[influence.palette_indices[slot] as usize];
            let weight = influence.weights[slot];
            pos += skin_matrix.transform_point3(rest_vertex.pos) * weight;
            normal += skin_matrix.transform_vector3(rest_vertex.normal) * weight;
        }

        *target = VertexGeometry {
            pos,
            normal: normal.try_normalize().unwrap_or(Vec3::Y),
        };
    }
}

#[cfg(test)]
mod tests {
    use glam::{UVec4, Vec4};

    use super::*;
    use crate::skinning::apply_skinning;

    const TOLERANCE: f32 = 1e-5;

    fn two_bone_skeleton() -> Skeleton {
        let mut skeleton = Skeleton::new("test");
        let root = skeleton.add_bone("root", None);
        skeleton.add_bone("child", Some(root));
        skeleton
    }

    fn posed_globals() -> Vec<Mat4> {
        vec![
            Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0))
                * Mat4::from_rotation_z(37.0_f32.to_radians()),
        ]
    }

    fn skin_with_every_influence_case() -> SkinData {
        SkinData {
            skeleton_id: 0,
            bone_indices: vec![
                UVec4::new(0, 0, 0, 0),
                UVec4::new(0, 1, 0, 0),
                UVec4::new(1, 0, 0, 0),
                UVec4::new(1, 0, 0, 0),
                UVec4::new(7, 0, 0, 0),
                UVec4::new(0, 9, 0, 0),
                UVec4::new(0, 0, 0, 0),
            ],
            bone_weights: vec![
                Vec4::new(1.0, 0.0, 0.0, 0.0),
                Vec4::new(0.3, 0.7, 0.0, 0.0),
                Vec4::new(0.6, 0.4, 0.0, 0.0),
                Vec4::new(0.0, 0.0, 0.0, 0.0),
                Vec4::new(0.0, 0.0, 0.0, 0.0),
                Vec4::new(0.5, 0.5, 0.0, 0.0),
                Vec4::new(1.0, 0.0, 0.0, 0.0),
            ],
            base_positions: vec![
                Vec3::new(0.2, 1.0, -0.4),
                Vec3::new(-0.7, 2.0, 0.3),
                Vec3::new(0.5, 3.0, 0.9),
                Vec3::new(0.1, 4.0, -0.6),
                Vec3::new(0.3, 5.0, 0.2),
                Vec3::new(-0.2, 6.0, 0.8),
                Vec3::new(0.0, 0.0, 0.0),
            ],
            base_normals: vec![
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(0.6, 0.8, 0.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 2.0),
                Vec3::new(0.0, 0.0, 0.0),
            ],
        }
    }

    #[test]
    fn skin_geometry_matches_apply_skinning() {
        let skeleton = two_bone_skeleton();
        let globals = posed_globals();
        let skin_data = skin_with_every_influence_case();
        let vertex_count = skin_data.base_positions.len();

        let mut expected_positions = vec![Vec3::ZERO; vertex_count];
        let mut expected_normals = vec![Vec3::Y; vertex_count];
        apply_skinning(
            &skin_data,
            &globals,
            &skeleton,
            &mut expected_positions,
            &mut expected_normals,
        );

        let mut skinned = vec![VertexGeometry::default(); vertex_count];
        skin_geometry(
            &collect_rest_geometry(&skin_data),
            &pack_skin_influences(&skin_data, skeleton.bone_count()),
            &build_skin_palette(&skeleton, &globals),
            &mut skinned,
        );

        for (vertex, geometry) in skinned.iter().enumerate() {
            assert!(
                geometry
                    .pos
                    .abs_diff_eq(expected_positions[vertex], TOLERANCE),
                "position at vertex {vertex}: {:?} vs {:?}",
                geometry.pos,
                expected_positions[vertex]
            );
            assert!(
                geometry
                    .normal
                    .abs_diff_eq(expected_normals[vertex], TOLERANCE),
                "normal at vertex {vertex}: {:?} vs {:?}",
                geometry.normal,
                expected_normals[vertex]
            );
        }
    }

    #[test]
    fn palette_ends_with_the_identity_entry() {
        let skeleton = two_bone_skeleton();
        let palette = build_skin_palette(&skeleton, &posed_globals());

        assert_eq!(palette.len(), skin_palette_len(skeleton.bone_count()));
        assert_eq!(palette[skeleton.bone_count()], Mat4::IDENTITY);
    }

    #[test]
    fn vertex_without_a_usable_bone_points_at_the_identity_entry() {
        let influence = pack_vertex_influence([7, 0, 0, 0], [0.0; 4], 2);

        assert_eq!(influence.palette_indices[0], 2);
        assert_eq!(influence.weights, [1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn skin_influence_is_packed_without_padding() {
        assert_eq!(std::mem::size_of::<SkinInfluence>(), 32);
        assert_eq!(std::mem::offset_of!(SkinInfluence, weights), 16);
    }
}
