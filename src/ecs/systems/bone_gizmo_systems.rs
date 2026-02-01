use anyhow::Result;
use cgmath::{Matrix4, SquareMatrix, Vector4};

use crate::animation::Skeleton;
use crate::debugview::gizmo::BoneGizmoData;
use crate::ecs::component::{ColorVertex, LineMesh};
use crate::render::RenderBackend;

const BONE_LINE_COLOR: [f32; 3] = [0.0, 0.8, 0.0];
const JOINT_MARKER_COLOR: [f32; 3] = [1.0, 1.0, 0.0];
const JOINT_CROSS_SIZE: f32 = 0.02;

pub fn compute_bone_local_offsets(
    skeleton: &Skeleton,
    rest_global_transforms: &[Matrix4<f32>],
) -> Vec<[f32; 3]> {
    skeleton
        .bones
        .iter()
        .enumerate()
        .map(|(idx, bone)| {
            let has_ibp = bone.inverse_bind_pose != Matrix4::identity();

            if has_ibp {
                if let Some(bind_global) = bone.inverse_bind_pose.invert() {
                    let bind_world_pos =
                        bind_global * Vector4::new(0.0, 0.0, 0.0, 1.0);

                    if idx < rest_global_transforms.len() {
                        if let Some(inv_rest) = rest_global_transforms[idx].invert() {
                            let local = inv_rest * bind_world_pos;
                            return [local.x, local.y, local.z];
                        }
                    }

                    return [bind_world_pos.x, bind_world_pos.y, bind_world_pos.z];
                }
            }

            [0.0, 0.0, 0.0]
        })
        .collect()
}

pub fn build_bone_line_mesh(
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    bone_local_offsets: &[[f32; 3]],
    mesh: &mut LineMesh,
) {
    mesh.vertices.clear();
    mesh.indices.clear();

    if global_transforms.is_empty() {
        return;
    }

    let display_transforms =
        compute_display_transforms(skeleton, global_transforms, bone_local_offsets);

    for bone in &skeleton.bones {
        let bone_idx = bone.id as usize;
        if bone_idx >= display_transforms.len() {
            continue;
        }

        let pos = display_transforms[bone_idx];

        if let Some(parent_id) = bone.parent_id {
            let parent_idx = parent_id as usize;
            if parent_idx >= display_transforms.len() {
                continue;
            }

            let parent_pos = display_transforms[parent_idx];

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

fn compute_display_transforms(
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    bone_local_offsets: &[[f32; 3]],
) -> Vec<[f32; 3]> {
    let inv_root = skeleton
        .root_transform
        .invert()
        .unwrap_or(Matrix4::identity());

    skeleton
        .bones
        .iter()
        .enumerate()
        .map(|(idx, _bone)| {
            if idx >= global_transforms.len() {
                return [0.0, 0.0, 0.0];
            }

            let offset = if idx < bone_local_offsets.len() {
                bone_local_offsets[idx]
            } else {
                [0.0, 0.0, 0.0]
            };

            let animated_world_pos = global_transforms[idx]
                * Vector4::new(offset[0], offset[1], offset[2], 1.0);
            let display_pos = inv_root * animated_world_pos;
            [display_pos.x, display_pos.y, display_pos.z]
        })
        .collect()
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
