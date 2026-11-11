use thyllore_math_core::scale_translation;

use crate::mesh::{MeshMorph, Vertex};
use crate::skeleton::Skeleton;
use crate::skin::SkinData;

pub fn scale_skeleton(skeleton: &mut Skeleton, factor: f32) {
    for bone in &mut skeleton.bones {
        bone.local_transform = scale_translation(&bone.local_transform, factor);
        bone.inverse_bind_pose = scale_translation(&bone.inverse_bind_pose, factor);
    }
    skeleton.root_transform = scale_translation(&skeleton.root_transform, factor);
}

pub fn scale_skin(skin: &mut SkinData, factor: f32) {
    for position in &mut skin.base_positions {
        *position *= factor;
    }
}

pub fn scale_morph(morph: &mut MeshMorph, factor: f32) {
    for channel in &mut morph.channels {
        for delta in &mut channel.position_deltas {
            delta.delta = delta.delta.map(|component| component * factor);
        }
    }
}

pub fn scale_vertex_positions(vertices: &mut [Vertex], factor: f32) {
    for vertex in vertices {
        vertex.pos = vertex.pos * factor;
    }
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec2, Vec3, Vec4};

    use super::*;
    use crate::mesh::{MorphChannel, SparseDelta};
    use crate::skeleton::Bone;

    fn translation(x: f32, y: f32, z: f32) -> Mat4 {
        Mat4::from_translation(Vec3::new(x, y, z))
    }

    #[test]
    fn skeleton_scale_moves_translations_and_keeps_rotations() {
        let mut skeleton = Skeleton::new("s");
        let rotation = Mat4::from_rotation_y(f32::to_radians(30.0));
        skeleton.bones.push(Bone {
            local_transform: rotation * translation(1.0, 2.0, 3.0),
            inverse_bind_pose: translation(-1.0, -2.0, -3.0),
            ..Default::default()
        });
        skeleton.root_transform = translation(0.0, 10.0, 0.0);

        scale_skeleton(&mut skeleton, 0.5);

        let bone = &skeleton.bones[0];
        let expected = rotation * translation(1.0, 2.0, 3.0);
        assert!(
            (bone.local_transform.w_axis.truncate() - expected.w_axis.truncate() * 0.5).length()
                < 1e-6
        );
        assert_eq!(bone.local_transform.x_axis, expected.x_axis);
        assert_eq!(
            bone.inverse_bind_pose.w_axis.truncate(),
            Vec3::new(-0.5, -1.0, -1.5)
        );
        assert_eq!(skeleton.root_transform.w_axis.y, 5.0);
    }

    #[test]
    fn skin_morph_and_vertices_scale_positions_only() {
        let mut skin = SkinData {
            base_positions: vec![Vec3::new(2.0, 4.0, 6.0)],
            base_normals: vec![Vec3::new(0.0, 1.0, 0.0)],
            ..Default::default()
        };
        let mut morph = MeshMorph {
            source_mesh: String::new(),
            channels: vec![MorphChannel {
                name: "a".into(),
                default_weight: 0.0,
                position_deltas: vec![SparseDelta {
                    vertex_index: 0,
                    delta: [1.0, 2.0, 3.0],
                }],
                normal_deltas: vec![SparseDelta {
                    vertex_index: 0,
                    delta: [1.0, 0.0, 0.0],
                }],
            }],
        };
        let mut vertices = vec![Vertex::new(
            Vec3::new(1.0, 1.0, 1.0),
            Vec4::new(1.0, 1.0, 1.0, 1.0),
            Vec2::new(0.5, 0.5),
        )];

        scale_skin(&mut skin, 2.0);
        scale_morph(&mut morph, 2.0);
        scale_vertex_positions(&mut vertices, 2.0);

        assert_eq!(skin.base_positions[0], Vec3::new(4.0, 8.0, 12.0));
        assert_eq!(skin.base_normals[0], Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(morph.channels[0].position_deltas[0].delta, [2.0, 4.0, 6.0]);
        assert_eq!(morph.channels[0].normal_deltas[0].delta, [1.0, 0.0, 0.0]);
        assert_eq!(vertices[0].pos, Vec3::new(2.0, 2.0, 2.0));
        assert_eq!(vertices[0].tex_coord, Vec2::new(0.5, 0.5));
    }
}
