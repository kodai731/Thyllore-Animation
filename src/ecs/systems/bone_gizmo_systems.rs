use anyhow::Result;
use cgmath::{Matrix4, SquareMatrix, Vector4};

use crate::animation::Skeleton;
use crate::debugview::gizmo::BoneGizmoData;
use crate::ecs::component::{ColorVertex, LineMesh};
use crate::render::RenderBackend;

const BONE_LINE_COLOR: [f32; 3] = [0.0, 0.8, 0.0];
const JOINT_MARKER_COLOR: [f32; 3] = [1.0, 1.0, 0.0];
const JOINT_CROSS_SIZE: f32 = 0.02;

const BONE_SOLID_COLOR: [f32; 3] = [0.2, 0.45, 0.7];
const BONE_WIRE_COLOR: [f32; 3] = [0.05, 0.15, 0.35];
const OCTA_WIDTH_RATIO: f32 = 0.1;
const OCTA_DEPTH_RATIO: f32 = 0.1;

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

pub fn build_octahedral_bone_meshes(
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    bone_local_offsets: &[[f32; 3]],
    solid_mesh: &mut LineMesh,
    wire_mesh: &mut LineMesh,
) {
    solid_mesh.vertices.clear();
    solid_mesh.indices.clear();
    wire_mesh.vertices.clear();
    wire_mesh.indices.clear();

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

        let Some(parent_id) = bone.parent_id else {
            continue;
        };
        let parent_idx = parent_id as usize;
        if parent_idx >= display_transforms.len() {
            continue;
        }

        let head = display_transforms[parent_idx];
        let tail = display_transforms[bone_idx];

        let Some((bone_length, bone_dir, right, forward)) = compute_bone_axes(head, tail) else {
            continue;
        };

        let w = bone_length * OCTA_WIDTH_RATIO;
        let d = bone_length * OCTA_DEPTH_RATIO;

        let mid = [
            head[0] + bone_dir[0] * d,
            head[1] + bone_dir[1] * d,
            head[2] + bone_dir[2] * d,
        ];

        let verts: [[f32; 3]; 6] = [
            head,
            [mid[0] + right[0] * w, mid[1] + right[1] * w, mid[2] + right[2] * w],
            [mid[0] + forward[0] * w, mid[1] + forward[1] * w, mid[2] + forward[2] * w],
            [mid[0] - right[0] * w, mid[1] - right[1] * w, mid[2] - right[2] * w],
            [mid[0] - forward[0] * w, mid[1] - forward[1] * w, mid[2] - forward[2] * w],
            tail,
        ];

        append_octahedral_solid(solid_mesh, &verts);
        append_octahedral_wire(wire_mesh, &verts);
    }
}

fn append_octahedral_solid(mesh: &mut LineMesh, verts: &[[f32; 3]; 6]) {
    let base = mesh.vertices.len() as u32;

    for v in verts {
        mesh.vertices.push(ColorVertex {
            pos: *v,
            color: BONE_SOLID_COLOR,
        });
    }

    let tris: [[u32; 3]; 8] = [
        [0, 1, 2],
        [0, 2, 3],
        [0, 3, 4],
        [0, 4, 1],
        [5, 2, 1],
        [5, 3, 2],
        [5, 4, 3],
        [5, 1, 4],
    ];

    for tri in &tris {
        mesh.indices.push(base + tri[0]);
        mesh.indices.push(base + tri[1]);
        mesh.indices.push(base + tri[2]);
    }
}

fn append_octahedral_wire(mesh: &mut LineMesh, verts: &[[f32; 3]; 6]) {
    let base = mesh.vertices.len() as u32;

    for v in verts {
        mesh.vertices.push(ColorVertex {
            pos: *v,
            color: BONE_WIRE_COLOR,
        });
    }

    let edges: [[u32; 2]; 12] = [
        [0, 1], [0, 2], [0, 3], [0, 4],
        [5, 1], [5, 2], [5, 3], [5, 4],
        [1, 2], [2, 3], [3, 4], [4, 1],
    ];

    for edge in &edges {
        mesh.indices.push(base + edge[0]);
        mesh.indices.push(base + edge[1]);
    }
}

fn compute_bone_axes(
    head: [f32; 3],
    tail: [f32; 3],
) -> Option<(f32, [f32; 3], [f32; 3], [f32; 3])> {
    let dx = tail[0] - head[0];
    let dy = tail[1] - head[1];
    let dz = tail[2] - head[2];
    let bone_length = (dx * dx + dy * dy + dz * dz).sqrt();

    if bone_length < 1e-6 {
        return None;
    }

    let inv_len = 1.0 / bone_length;
    let bone_dir = [dx * inv_len, dy * inv_len, dz * inv_len];

    let up_candidate = if bone_dir[1].abs() < 0.99 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };

    let right = cross(bone_dir, up_candidate);
    let right = normalize(right);
    let forward = cross(bone_dir, right);
    let forward = normalize(forward);

    Some((bone_length, bone_dir, right, forward))
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-10 {
        return [0.0, 0.0, 0.0];
    }
    let inv = 1.0 / len;
    [v[0] * inv, v[1] * inv, v[2] * inv]
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
    backend.update_or_create_line_buffers(&mut bone_gizmo.stick_mesh)
}
