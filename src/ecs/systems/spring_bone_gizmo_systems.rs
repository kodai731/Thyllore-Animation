use cgmath::{Matrix4, Vector3};

use crate::ecs::component::{ColliderShape, ColorVertex, LineMesh, SpringBoneSetup};
use crate::ecs::systems::bone_gizmo_systems::compute_display_transforms;

const COLOR_COLLIDER: [f32; 3] = [0.2, 0.8, 0.2];
const CIRCLE_SEGMENTS: u32 = 16;

pub fn build_spring_bone_gizmo_mesh(
    setup: &SpringBoneSetup,
    global_transforms: &[Matrix4<f32>],
    bone_local_offsets: &[[f32; 3]],
    mesh_scale: f32,
    mesh: &mut LineMesh,
) {
    mesh.vertices.clear();
    mesh.indices.clear();

    if global_transforms.is_empty() {
        return;
    }

    let display_positions =
        compute_display_transforms(global_transforms, bone_local_offsets, mesh_scale);

    for collider in &setup.colliders {
        let bone_idx = collider.bone_id as usize;
        if bone_idx >= display_positions.len() {
            continue;
        }

        let bone_pos = display_positions[bone_idx];
        let center = [
            bone_pos[0] + collider.offset.x * mesh_scale,
            bone_pos[1] + collider.offset.y * mesh_scale,
            bone_pos[2] + collider.offset.z * mesh_scale,
        ];

        match &collider.shape {
            ColliderShape::Sphere { radius } => {
                let scaled_radius = radius * mesh_scale;
                append_wire_sphere(mesh, center, scaled_radius, COLOR_COLLIDER, CIRCLE_SEGMENTS);
            }
            ColliderShape::Capsule { radius, tail } => {
                let scaled_radius = radius * mesh_scale;
                let tail_pos = [
                    bone_pos[0] + tail.x * mesh_scale,
                    bone_pos[1] + tail.y * mesh_scale,
                    bone_pos[2] + tail.z * mesh_scale,
                ];
                append_wire_capsule(
                    mesh,
                    center,
                    tail_pos,
                    scaled_radius,
                    COLOR_COLLIDER,
                    CIRCLE_SEGMENTS,
                );
            }
        }
    }
}

fn append_wire_sphere(
    mesh: &mut LineMesh,
    center: [f32; 3],
    radius: f32,
    color: [f32; 3],
    segments: u32,
) {
    append_wire_circle(mesh, center, radius, color, segments, Plane::XY);
    append_wire_circle(mesh, center, radius, color, segments, Plane::XZ);
    append_wire_circle(mesh, center, radius, color, segments, Plane::YZ);
}

fn append_wire_capsule(
    mesh: &mut LineMesh,
    center: [f32; 3],
    tail: [f32; 3],
    radius: f32,
    color: [f32; 3],
    segments: u32,
) {
    append_wire_sphere(mesh, center, radius, color, segments);
    append_wire_sphere(mesh, tail, radius, color, segments);

    let dx = tail[0] - center[0];
    let dy = tail[1] - center[1];
    let dz = tail[2] - center[2];
    let len = (dx * dx + dy * dy + dz * dz).sqrt();

    if len < 1e-6 {
        return;
    }

    let dir = [dx / len, dy / len, dz / len];
    let (right, up) = compute_perpendicular_axes(dir);

    let offsets = [
        [right[0] * radius, right[1] * radius, right[2] * radius],
        [-right[0] * radius, -right[1] * radius, -right[2] * radius],
        [up[0] * radius, up[1] * radius, up[2] * radius],
        [-up[0] * radius, -up[1] * radius, -up[2] * radius],
    ];

    for offset in &offsets {
        let from = [
            center[0] + offset[0],
            center[1] + offset[1],
            center[2] + offset[2],
        ];
        let to = [
            tail[0] + offset[0],
            tail[1] + offset[1],
            tail[2] + offset[2],
        ];
        append_solid_line(mesh, from, to, color);
    }
}

#[derive(Clone, Copy)]
enum Plane {
    XY,
    XZ,
    YZ,
}

fn append_wire_circle(
    mesh: &mut LineMesh,
    center: [f32; 3],
    radius: f32,
    color: [f32; 3],
    segments: u32,
    plane: Plane,
) {
    let seg = segments.max(4);

    for i in 0..seg {
        let angle0 = (i as f32 / seg as f32) * std::f32::consts::TAU;
        let angle1 = ((i + 1) as f32 / seg as f32) * std::f32::consts::TAU;

        let (c0, s0) = (angle0.cos(), angle0.sin());
        let (c1, s1) = (angle1.cos(), angle1.sin());

        let (p0, p1) = match plane {
            Plane::XY => (
                [center[0] + c0 * radius, center[1] + s0 * radius, center[2]],
                [center[0] + c1 * radius, center[1] + s1 * radius, center[2]],
            ),
            Plane::XZ => (
                [center[0] + c0 * radius, center[1], center[2] + s0 * radius],
                [center[0] + c1 * radius, center[1], center[2] + s1 * radius],
            ),
            Plane::YZ => (
                [center[0], center[1] + c0 * radius, center[2] + s0 * radius],
                [center[0], center[1] + c1 * radius, center[2] + s1 * radius],
            ),
        };

        append_solid_line(mesh, p0, p1, color);
    }
}

fn append_solid_line(mesh: &mut LineMesh, from: [f32; 3], to: [f32; 3], color: [f32; 3]) {
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(ColorVertex { pos: from, color });
    mesh.vertices.push(ColorVertex { pos: to, color });
    mesh.indices.push(base);
    mesh.indices.push(base + 1);
}

fn compute_perpendicular_axes(dir: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let up_hint = if dir[1].abs() < 0.99 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };

    let right = normalize(cross(dir, up_hint));
    let up = normalize(cross(dir, right));
    (right, up)
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
