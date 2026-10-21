use std::collections::HashSet;

use cgmath::{Matrix4, Vector3};
use thyllore_anim_core::spring_bone::chains_under_bone;

use crate::animation::{BoneId, Skeleton};
use crate::ecs::component::{
    ColliderShape, ColorVertex, LineMesh, SpringBoneSetup, SpringChain, SpringChainId,
};
use crate::ecs::resource::gizmo::{BoneSelectionState, SpringChainGizmoMode};
use crate::ecs::systems::bone_gizmo_systems::compute_display_transforms;

const COLOR_COLLIDER: [f32; 3] = [0.2, 0.8, 0.2];
const COLOR_CHAIN: [f32; 3] = [0.95, 0.75, 0.2];
const CIRCLE_SEGMENTS: u32 = 16;
const CHAIN_RADIUS_SEGMENTS: u32 = 8;

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

/// The chains holding the active bone: the only ones that show their joint hit radii.
pub fn chains_with_radius(
    setup: &SpringBoneSetup,
    selection: &BoneSelectionState,
    mode: SpringChainGizmoMode,
) -> Vec<SpringChainId> {
    let Some(active) = selection.active_bone_index else {
        return Vec::new();
    };
    if mode == SpringChainGizmoMode::None {
        return Vec::new();
    }
    let active_only = HashSet::from([active]);
    setup
        .chains
        .iter()
        .filter(|chain| chain_has_selected_joint(chain, &active_only))
        .map(|chain| chain.id)
        .collect()
}

/// Chains whose joint is selected, plus every chain hanging directly under a selected bone.
pub fn chains_to_draw(
    setup: &SpringBoneSetup,
    skeleton: &Skeleton,
    selection: &BoneSelectionState,
    mode: SpringChainGizmoMode,
) -> Vec<SpringChainId> {
    match mode {
        SpringChainGizmoMode::None => Vec::new(),
        SpringChainGizmoMode::All => setup.chains.iter().map(|chain| chain.id).collect(),
        SpringChainGizmoMode::Selected => {
            let selected = &selection.selected_bone_indices;
            let mut ids: HashSet<SpringChainId> = setup
                .chains
                .iter()
                .filter(|chain| chain_has_selected_joint(chain, selected))
                .map(|chain| chain.id)
                .collect();
            for &bone_index in selected {
                ids.extend(chains_under_bone(setup, skeleton, bone_index as BoneId));
            }
            let mut ordered: Vec<SpringChainId> = ids.into_iter().collect();
            ordered.sort_unstable();
            ordered
        }
    }
}

fn chain_has_selected_joint(chain: &SpringChain, selected: &HashSet<usize>) -> bool {
    chain
        .joints
        .iter()
        .any(|joint| selected.contains(&(joint.bone_id as usize)))
}

pub fn build_spring_bone_gizmo_mesh(
    setup: &SpringBoneSetup,
    shown: &SpringGizmoSelection,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::Bone;
    use crate::ecs::component::SpringJointParam;

    fn skeleton_with_two_chains() -> (Skeleton, SpringBoneSetup) {
        let mut skeleton = Skeleton::default();
        let parents = [None, Some(0), Some(1), Some(1), Some(3)];
        for (id, parent_id) in parents.iter().enumerate() {
            skeleton.bones.push(Bone {
                id: id as BoneId,
                parent_id: *parent_id,
                ..Bone::default()
            });
        }
        for id in 1..5u32 {
            let parent = skeleton.bones[id as usize].parent_id.expect("child bone");
            skeleton.bones[parent as usize].children.push(id);
        }

        let chain = |id: SpringChainId, bone_ids: &[BoneId]| SpringChain {
            id,
            joints: bone_ids
                .iter()
                .map(|&bone_id| SpringJointParam {
                    bone_id,
                    ..SpringJointParam::default()
                })
                .collect(),
            ..Default::default()
        };
        let setup = SpringBoneSetup {
            chains: vec![chain(10, &[2]), chain(11, &[3, 4])],
            ..Default::default()
        };
        (skeleton, setup)
    }

    fn selecting(indices: &[usize]) -> BoneSelectionState {
        BoneSelectionState {
            selected_bone_indices: indices.iter().copied().collect(),
            active_bone_index: indices.first().copied(),
        }
    }

    #[test]
    fn selected_mode_draws_chains_under_the_selected_bone() {
        let (skeleton, setup) = skeleton_with_two_chains();
        let ids = chains_to_draw(
            &setup,
            &skeleton,
            &selecting(&[1]),
            SpringChainGizmoMode::Selected,
        );
        assert_eq!(ids, vec![10, 11]);
    }

    #[test]
    fn selected_mode_draws_the_chain_of_a_selected_joint() {
        let (skeleton, setup) = skeleton_with_two_chains();
        let ids = chains_to_draw(
            &setup,
            &skeleton,
            &selecting(&[4]),
            SpringChainGizmoMode::Selected,
        );
        assert_eq!(ids, vec![11]);
    }

    #[test]
    fn radii_follow_the_active_bone_only() {
        let (_, setup) = skeleton_with_two_chains();
        let head_active = BoneSelectionState {
            selected_bone_indices: [1, 2, 3, 4].into_iter().collect(),
            active_bone_index: Some(1),
        };
        assert!(
            chains_with_radius(&setup, &head_active, SpringChainGizmoMode::Selected).is_empty()
        );
        assert_eq!(
            chains_with_radius(&setup, &selecting(&[4]), SpringChainGizmoMode::All),
            vec![11]
        );
    }

    #[test]
    fn all_and_none_ignore_the_selection() {
        let (skeleton, setup) = skeleton_with_two_chains();
        let none = selecting(&[]);
        assert_eq!(
            chains_to_draw(&setup, &skeleton, &none, SpringChainGizmoMode::All),
            vec![10, 11]
        );
        assert!(chains_to_draw(&setup, &skeleton, &none, SpringChainGizmoMode::None).is_empty());
        assert!(
            chains_to_draw(&setup, &skeleton, &none, SpringChainGizmoMode::Selected).is_empty()
        );
    }
}
