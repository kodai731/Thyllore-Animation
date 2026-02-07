use anyhow::Result;
use cgmath::{Matrix4, SquareMatrix, Vector3, Vector4};

use crate::animation::Skeleton;
use crate::debugview::gizmo::{BoneGizmoData, BoneSelectionState};
use crate::ecs::component::{ColorVertex, LineMesh};
use crate::math::ray_to_triangle_intersection;
use crate::render::RenderBackend;

const BONE_LINE_COLOR: [f32; 3] = [0.0, 0.8, 0.0];
const JOINT_MARKER_COLOR: [f32; 3] = [1.0, 1.0, 0.0];
const JOINT_CROSS_SIZE: f32 = 0.02;

const BONE_SOLID_COLOR: [f32; 3] = [0.2, 0.45, 0.7];
const BONE_WIRE_COLOR: [f32; 3] = [0.05, 0.15, 0.35];
const BONE_SELECTED_SOLID_COLOR: [f32; 3] = [0.4, 0.7, 1.0];
const BONE_SELECTED_WIRE_COLOR: [f32; 3] = [0.1, 0.3, 0.55];
const BONE_ACTIVE_SOLID_COLOR: [f32; 3] = [1.0, 0.6, 0.2];
const BONE_ACTIVE_WIRE_COLOR: [f32; 3] = [0.5, 0.3, 0.1];
const OCTA_WIDTH_RATIO: f32 = 0.1;
const OCTA_DEPTH_RATIO: f32 = 0.1;
const BOX_WIDTH_RATIO: f32 = 0.08;
const SPHERE_RADIUS_RATIO: f32 = 0.06;
const SPHERE_RING_COUNT: usize = 6;
const SPHERE_SEGMENT_COUNT: usize = 8;

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
    let default_selection = BoneSelectionState::default();
    build_octahedral_bone_meshes_with_selection(
        skeleton,
        global_transforms,
        bone_local_offsets,
        &default_selection,
        1.0,
        solid_mesh,
        wire_mesh,
    );
}

pub fn build_octahedral_bone_meshes_with_selection(
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    bone_local_offsets: &[[f32; 3]],
    selection: &BoneSelectionState,
    visual_scale: f32,
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

    let display_transforms = compute_display_transforms(
        skeleton,
        global_transforms,
        bone_local_offsets,
    );

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

        let Some((bone_length, bone_dir, right, forward)) =
            compute_bone_axes(head, tail)
        else {
            continue;
        };

        let w = bone_length * OCTA_WIDTH_RATIO * visual_scale;
        let d = bone_length * OCTA_DEPTH_RATIO * visual_scale;

        let mid = [
            head[0] + bone_dir[0] * d,
            head[1] + bone_dir[1] * d,
            head[2] + bone_dir[2] * d,
        ];

        let verts: [[f32; 3]; 6] = [
            head,
            [
                mid[0] + right[0] * w,
                mid[1] + right[1] * w,
                mid[2] + right[2] * w,
            ],
            [
                mid[0] + forward[0] * w,
                mid[1] + forward[1] * w,
                mid[2] + forward[2] * w,
            ],
            [
                mid[0] - right[0] * w,
                mid[1] - right[1] * w,
                mid[2] - right[2] * w,
            ],
            [
                mid[0] - forward[0] * w,
                mid[1] - forward[1] * w,
                mid[2] - forward[2] * w,
            ],
            tail,
        ];

        let (solid_color, wire_color) =
            resolve_bone_colors(bone_idx, selection);
        append_octahedral_solid_colored(
            solid_mesh,
            &verts,
            solid_color,
        );
        append_octahedral_wire_colored(
            wire_mesh,
            &verts,
            wire_color,
        );
    }
}

fn resolve_bone_colors(
    bone_index: usize,
    selection: &BoneSelectionState,
) -> ([f32; 3], [f32; 3]) {
    if selection.active_bone_index == Some(bone_index) {
        return (BONE_ACTIVE_SOLID_COLOR, BONE_ACTIVE_WIRE_COLOR);
    }

    if selection.selected_bone_indices.contains(&bone_index) {
        return (
            BONE_SELECTED_SOLID_COLOR,
            BONE_SELECTED_WIRE_COLOR,
        );
    }

    (BONE_SOLID_COLOR, BONE_WIRE_COLOR)
}

fn append_octahedral_solid_colored(
    mesh: &mut LineMesh,
    verts: &[[f32; 3]; 6],
    color: [f32; 3],
) {
    let base = mesh.vertices.len() as u32;

    for v in verts {
        mesh.vertices.push(ColorVertex {
            pos: *v,
            color,
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

fn append_octahedral_wire_colored(
    mesh: &mut LineMesh,
    verts: &[[f32; 3]; 6],
    color: [f32; 3],
) {
    let base = mesh.vertices.len() as u32;

    for v in verts {
        mesh.vertices.push(ColorVertex {
            pos: *v,
            color,
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

pub(crate) fn compute_display_transforms(
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

const OCTAHEDRAL_TRIANGLES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [0, 2, 3],
    [0, 3, 4],
    [0, 4, 1],
    [5, 2, 1],
    [5, 3, 2],
    [5, 4, 3],
    [5, 1, 4],
];

pub fn compute_octahedral_vertices_per_bone(
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    bone_local_offsets: &[[f32; 3]],
) -> Vec<(usize, [[f32; 3]; 6])> {
    if global_transforms.is_empty() {
        return Vec::new();
    }

    let display_transforms = compute_display_transforms(
        skeleton,
        global_transforms,
        bone_local_offsets,
    );

    let mut result = Vec::new();

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

        let Some((bone_length, bone_dir, right, forward)) =
            compute_bone_axes(head, tail)
        else {
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
            [
                mid[0] + right[0] * w,
                mid[1] + right[1] * w,
                mid[2] + right[2] * w,
            ],
            [
                mid[0] + forward[0] * w,
                mid[1] + forward[1] * w,
                mid[2] + forward[2] * w,
            ],
            [
                mid[0] - right[0] * w,
                mid[1] - right[1] * w,
                mid[2] - right[2] * w,
            ],
            [
                mid[0] - forward[0] * w,
                mid[1] - forward[1] * w,
                mid[2] - forward[2] * w,
            ],
            tail,
        ];

        result.push((bone_idx, verts));
    }

    result
}

pub fn select_bone_by_ray(
    ray_origin: Vector3<f32>,
    ray_direction: Vector3<f32>,
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    bone_local_offsets: &[[f32; 3]],
) -> Option<(usize, f32)> {
    let bone_verts = compute_octahedral_vertices_per_bone(
        skeleton,
        global_transforms,
        bone_local_offsets,
    );

    let mut closest: Option<(usize, f32)> = None;

    for (bone_idx, verts) in &bone_verts {
        for tri_indices in &OCTAHEDRAL_TRIANGLES {
            let v0 = verts[tri_indices[0]];
            let v1 = verts[tri_indices[1]];
            let v2 = verts[tri_indices[2]];

            if let Some(t) = ray_to_triangle_intersection(
                ray_origin,
                ray_direction,
                Vector3::new(v0[0], v0[1], v0[2]),
                Vector3::new(v1[0], v1[1], v1[2]),
                Vector3::new(v2[0], v2[1], v2[2]),
            ) {
                let is_closer = closest
                    .map_or(true, |(_, prev_t)| t < prev_t);
                if is_closer {
                    closest = Some((*bone_idx, t));
                }
            }
        }
    }

    closest
}

pub unsafe fn update_bone_gizmo_buffers(
    bone_gizmo: &mut BoneGizmoData,
    backend: &mut dyn RenderBackend,
) -> Result<()> {
    backend.update_or_create_line_buffers(&mut bone_gizmo.stick_mesh)
}

pub fn build_box_bone_meshes_with_selection(
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    bone_local_offsets: &[[f32; 3]],
    selection: &BoneSelectionState,
    visual_scale: f32,
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

        let Some((bone_length, _bone_dir, right, forward)) =
            compute_bone_axes(head, tail)
        else {
            continue;
        };

        let w = bone_length * BOX_WIDTH_RATIO * visual_scale;
        let box_verts = compute_box_vertices(head, tail, right, forward, w);

        let (solid_color, wire_color) = resolve_bone_colors(bone_idx, selection);
        append_box_solid_colored(solid_mesh, &box_verts, solid_color);
        append_box_wire_colored(wire_mesh, &box_verts, wire_color);
    }
}

fn compute_box_vertices(
    head: [f32; 3],
    tail: [f32; 3],
    right: [f32; 3],
    forward: [f32; 3],
    w: f32,
) -> [[f32; 3]; 8] {
    let mut verts = [[0.0f32; 3]; 8];

    let signs: [[f32; 2]; 4] = [
        [1.0, 1.0],
        [-1.0, 1.0],
        [-1.0, -1.0],
        [1.0, -1.0],
    ];

    for (i, [sr, sf]) in signs.iter().enumerate() {
        for k in 0..3 {
            verts[i][k] = head[k] + right[k] * w * sr + forward[k] * w * sf;
            verts[i + 4][k] = tail[k] + right[k] * w * sr + forward[k] * w * sf;
        }
    }

    verts
}

fn append_box_solid_colored(
    mesh: &mut LineMesh,
    verts: &[[f32; 3]; 8],
    color: [f32; 3],
) {
    let base = mesh.vertices.len() as u32;

    for v in verts {
        mesh.vertices.push(ColorVertex { pos: *v, color });
    }

    let tris: [[u32; 3]; 12] = [
        [0, 1, 2], [0, 2, 3],
        [4, 6, 5], [4, 7, 6],
        [0, 4, 5], [0, 5, 1],
        [2, 6, 7], [2, 7, 3],
        [1, 5, 6], [1, 6, 2],
        [0, 3, 7], [0, 7, 4],
    ];

    for tri in &tris {
        mesh.indices.push(base + tri[0]);
        mesh.indices.push(base + tri[1]);
        mesh.indices.push(base + tri[2]);
    }
}

fn append_box_wire_colored(
    mesh: &mut LineMesh,
    verts: &[[f32; 3]; 8],
    color: [f32; 3],
) {
    let base = mesh.vertices.len() as u32;

    for v in verts {
        mesh.vertices.push(ColorVertex { pos: *v, color });
    }

    let edges: [[u32; 2]; 12] = [
        [0, 1], [1, 2], [2, 3], [3, 0],
        [4, 5], [5, 6], [6, 7], [7, 4],
        [0, 4], [1, 5], [2, 6], [3, 7],
    ];

    for edge in &edges {
        mesh.indices.push(base + edge[0]);
        mesh.indices.push(base + edge[1]);
    }
}

pub fn build_sphere_bone_meshes_with_selection(
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    bone_local_offsets: &[[f32; 3]],
    selection: &BoneSelectionState,
    visual_scale: f32,
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

        let Some((bone_length, bone_dir, right, forward)) =
            compute_bone_axes(head, tail)
        else {
            continue;
        };

        let center = [
            (head[0] + tail[0]) * 0.5,
            (head[1] + tail[1]) * 0.5,
            (head[2] + tail[2]) * 0.5,
        ];
        let radius = bone_length * SPHERE_RADIUS_RATIO * visual_scale;

        let (solid_color, wire_color) = resolve_bone_colors(bone_idx, selection);
        append_sphere_solid_colored(
            solid_mesh, center, radius, bone_dir, right, forward, solid_color,
        );
        append_sphere_wire_colored(
            wire_mesh, center, radius, bone_dir, right, forward, wire_color,
        );
    }
}

fn sphere_point(
    center: [f32; 3],
    radius: f32,
    bone_dir: [f32; 3],
    right: [f32; 3],
    forward: [f32; 3],
    theta: f32,
    phi: f32,
) -> [f32; 3] {
    let st = theta.sin();
    let ct = theta.cos();
    let sp = phi.sin();
    let cp = phi.cos();

    let x = st * cp;
    let y = ct;
    let z = st * sp;

    [
        center[0] + radius * (right[0] * x + bone_dir[0] * y + forward[0] * z),
        center[1] + radius * (right[1] * x + bone_dir[1] * y + forward[1] * z),
        center[2] + radius * (right[2] * x + bone_dir[2] * y + forward[2] * z),
    ]
}

fn append_sphere_solid_colored(
    mesh: &mut LineMesh,
    center: [f32; 3],
    radius: f32,
    bone_dir: [f32; 3],
    right: [f32; 3],
    forward: [f32; 3],
    color: [f32; 3],
) {
    let rings = SPHERE_RING_COUNT;
    let segments = SPHERE_SEGMENT_COUNT;
    let base = mesh.vertices.len() as u32;

    for ring in 0..=rings {
        let theta = std::f32::consts::PI * ring as f32 / rings as f32;
        for seg in 0..=segments {
            let phi = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
            let pos = sphere_point(center, radius, bone_dir, right, forward, theta, phi);
            mesh.vertices.push(ColorVertex { pos, color });
        }
    }

    let cols = (segments + 1) as u32;
    for ring in 0..rings as u32 {
        for seg in 0..segments as u32 {
            let tl = base + ring * cols + seg;
            let tr = tl + 1;
            let bl = tl + cols;
            let br = bl + 1;

            mesh.indices.push(tl);
            mesh.indices.push(bl);
            mesh.indices.push(tr);

            mesh.indices.push(tr);
            mesh.indices.push(bl);
            mesh.indices.push(br);
        }
    }
}

fn append_sphere_wire_colored(
    mesh: &mut LineMesh,
    center: [f32; 3],
    radius: f32,
    bone_dir: [f32; 3],
    right: [f32; 3],
    forward: [f32; 3],
    color: [f32; 3],
) {
    let rings = SPHERE_RING_COUNT;
    let segments = SPHERE_SEGMENT_COUNT;

    for ring in 1..rings {
        let theta = std::f32::consts::PI * ring as f32 / rings as f32;
        let mut prev = sphere_point(center, radius, bone_dir, right, forward, theta, 0.0);

        for seg in 1..=segments {
            let phi = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
            let curr =
                sphere_point(center, radius, bone_dir, right, forward, theta, phi);

            let base = mesh.vertices.len() as u32;
            mesh.vertices.push(ColorVertex { pos: prev, color });
            mesh.vertices.push(ColorVertex { pos: curr, color });
            mesh.indices.push(base);
            mesh.indices.push(base + 1);

            prev = curr;
        }
    }

    for seg in 0..segments {
        let phi = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
        let mut prev = sphere_point(center, radius, bone_dir, right, forward, 0.0, phi);

        for ring in 1..=rings {
            let theta = std::f32::consts::PI * ring as f32 / rings as f32;
            let curr =
                sphere_point(center, radius, bone_dir, right, forward, theta, phi);

            let base = mesh.vertices.len() as u32;
            mesh.vertices.push(ColorVertex { pos: prev, color });
            mesh.vertices.push(ColorVertex { pos: curr, color });
            mesh.indices.push(base);
            mesh.indices.push(base + 1);

            prev = curr;
        }
    }
}
