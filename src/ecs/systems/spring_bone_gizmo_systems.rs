use glam::{Mat4, Vec3};

use crate::ecs::component::{ColorVertex, LineMesh, SpringBoneSetup, SpringChain, SpringChainId};
use crate::ecs::systems::bone_gizmo_systems::compute_display_transforms;
use thyllore_anim_core::spring_bone::{
    world_collider, CHAIN_RADIUS_SEGMENTS, CIRCLE_SEGMENTS, COLOR_CHAIN, COLOR_COLLIDER,
};
use thyllore_math_core::{wire_capsule_segments, wire_sphere_segments};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColliderGizmo {
    Shown,
    Hidden,
}

pub struct SpringGizmoSelection {
    pub chain_ids: Vec<SpringChainId>,
    pub radius_chain_ids: Vec<SpringChainId>,
    pub colliders: ColliderGizmo,
}

pub fn build_spring_bone_gizmo_mesh(
    setup: &SpringBoneSetup,
    shown: &SpringGizmoSelection,
    global_transforms: &[Mat4],
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

    for chain in setup
        .chains
        .iter()
        .filter(|chain| shown.chain_ids.contains(&chain.id))
    {
        let radii = if shown.radius_chain_ids.contains(&chain.id) {
            JointRadii::Shown
        } else {
            JointRadii::Hidden
        };
        append_chain(mesh, chain, &display_positions, mesh_scale, radii);
    }

    if shown.colliders == ColliderGizmo::Hidden {
        return;
    }

    for collider in &setup.colliders {
        let bone_idx = collider.bone_id as usize;
        if bone_idx >= global_transforms.len() {
            continue;
        }

        let placed = world_collider(collider, &global_transforms[bone_idx]);
        let to_display = |point: Vec3| (point * mesh_scale).into();
        let center = to_display(placed.center);
        let radius = placed.radius * mesh_scale;

        match placed.tail {
            None => append_wire_sphere(mesh, center, radius, COLOR_COLLIDER, CIRCLE_SEGMENTS),
            Some(tail) => append_wire_capsule(
                mesh,
                center,
                to_display(tail),
                radius,
                COLOR_COLLIDER,
                CIRCLE_SEGMENTS,
            ),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JointRadii {
    Shown,
    Hidden,
}

fn append_chain(
    mesh: &mut LineMesh,
    chain: &SpringChain,
    display_positions: &[[f32; 3]],
    mesh_scale: f32,
    radii: JointRadii,
) {
    let joint_positions: Vec<([f32; 3], f32)> = chain
        .joints
        .iter()
        .filter_map(|joint| {
            let position = display_positions.get(joint.bone_id as usize)?;
            Some((*position, joint.hit_radius * mesh_scale))
        })
        .collect();

    for pair in joint_positions.windows(2) {
        append_solid_line(mesh, pair[0].0, pair[1].0, COLOR_CHAIN);
    }
    if radii == JointRadii::Hidden {
        return;
    }
    for (position, radius) in &joint_positions {
        append_wire_sphere(mesh, *position, *radius, COLOR_CHAIN, CHAIN_RADIUS_SEGMENTS);
    }
}

fn append_wire_sphere(
    mesh: &mut LineMesh,
    center: [f32; 3],
    radius: f32,
    color: [f32; 3],
    segments: u32,
) {
    for (from, to) in wire_sphere_segments(center, radius, segments) {
        append_solid_line(mesh, from, to, color);
    }
}

fn append_wire_capsule(
    mesh: &mut LineMesh,
    center: [f32; 3],
    tail: [f32; 3],
    radius: f32,
    color: [f32; 3],
    segments: u32,
) {
    for (from, to) in wire_capsule_segments(center, tail, radius, segments) {
        append_solid_line(mesh, from, to, color);
    }
}

fn append_solid_line(mesh: &mut LineMesh, from: [f32; 3], to: [f32; 3], color: [f32; 3]) {
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(ColorVertex { pos: from, color });
    mesh.vertices.push(ColorVertex { pos: to, color });
    mesh.indices.push(base);
    mesh.indices.push(base + 1);
}
