use anyhow::Result;
use cgmath::Matrix4;

use crate::animation::Skeleton;
use crate::debugview::gizmo::BoneGizmoData;
use crate::ecs::component::{ColorVertex, LineMesh};
use crate::render::RenderBackend;

const BONE_LINE_COLOR: [f32; 3] = [0.0, 0.8, 0.0];
const JOINT_MARKER_COLOR: [f32; 3] = [1.0, 1.0, 0.0];
const JOINT_CROSS_SIZE: f32 = 0.005;

pub fn build_bone_line_mesh(
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    mesh: &mut LineMesh,
) {
    mesh.vertices.clear();
    mesh.indices.clear();

    if global_transforms.is_empty() {
        return;
    }

    for bone in &skeleton.bones {
        let bone_idx = bone.id as usize;
        if bone_idx >= global_transforms.len() {
            continue;
        }

        let transform = &global_transforms[bone_idx];
        let pos = [transform[3][0], transform[3][1], transform[3][2]];

        if let Some(parent_id) = bone.parent_id {
            let parent_idx = parent_id as usize;
            if parent_idx >= global_transforms.len() {
                continue;
            }

            let parent_transform = &global_transforms[parent_idx];
            let parent_pos = [
                parent_transform[3][0],
                parent_transform[3][1],
                parent_transform[3][2],
            ];

            let base = mesh.vertices.len() as u32;
            mesh.vertices.push(ColorVertex {
                pos: parent_pos,
                color: BONE_LINE_COLOR,
            });
            mesh.vertices.push(ColorVertex {
                pos,
                color: BONE_LINE_COLOR,
            });
            mesh.indices.push(base);
            mesh.indices.push(base + 1);
        }

        append_joint_cross_marker(mesh, pos);
    }
}

fn append_joint_cross_marker(mesh: &mut LineMesh, pos: [f32; 3]) {
    let s = JOINT_CROSS_SIZE;
    let offsets = [
        ([-s, 0.0, 0.0], [s, 0.0, 0.0]),
        ([0.0, -s, 0.0], [0.0, s, 0.0]),
        ([0.0, 0.0, -s], [0.0, 0.0, s]),
    ];

    for (neg, posi) in &offsets {
        let base = mesh.vertices.len() as u32;

        mesh.vertices.push(ColorVertex {
            pos: [pos[0] + neg[0], pos[1] + neg[1], pos[2] + neg[2]],
            color: JOINT_MARKER_COLOR,
        });
        mesh.vertices.push(ColorVertex {
            pos: [pos[0] + posi[0], pos[1] + posi[1], pos[2] + posi[2]],
            color: JOINT_MARKER_COLOR,
        });

        mesh.indices.push(base);
        mesh.indices.push(base + 1);
    }
}

pub unsafe fn update_bone_gizmo_buffers(
    bone_gizmo: &mut BoneGizmoData,
    backend: &mut dyn RenderBackend,
) -> Result<()> {
    backend.update_or_create_line_buffers(&mut bone_gizmo.mesh)
}
