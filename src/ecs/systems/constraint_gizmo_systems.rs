use crate::animation::{
    BoneId, ConstraintType, Skeleton,
};
use crate::ecs::component::{ColorVertex, LineMesh};
use crate::ecs::component::ConstraintSet;
use crate::ecs::systems::bone_gizmo_systems::compute_display_transforms;

use cgmath::Matrix4;

const COLOR_POSITION: [f32; 3] = [0.0, 0.8, 0.8];
const COLOR_ROTATION: [f32; 3] = [0.2, 0.9, 0.2];
const COLOR_SCALE: [f32; 3] = [0.9, 0.9, 0.2];
const COLOR_PARENT: [f32; 3] = [0.9, 0.2, 0.9];
const COLOR_AIM: [f32; 3] = [1.0, 0.5, 0.1];
const COLOR_IK_CHAIN: [f32; 3] = [1.0, 0.3, 0.3];
const COLOR_IK_TARGET: [f32; 3] = [0.7, 0.3, 1.0];

const DASH_SEGMENTS: u32 = 8;
const ARROW_SIZE: f32 = 0.02;

pub fn build_constraint_gizmo_mesh(
    constraint_set: &ConstraintSet,
    skeleton: &Skeleton,
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

    let display_positions = compute_display_transforms(
        global_transforms,
        bone_local_offsets,
        mesh_scale,
    );

    for entry in super::constraint_set_systems::constraint_set_enabled(constraint_set) {
        match &entry.constraint {
            ConstraintType::Position(data) => {
                append_position_constraint_lines(
                    data.constrained_bone,
                    data.target_bone,
                    &display_positions,
                    mesh,
                );
            }
            ConstraintType::Rotation(data) => {
                append_rotation_constraint_lines(
                    data.constrained_bone,
                    data.target_bone,
                    &display_positions,
                    mesh,
                );
            }
            ConstraintType::Scale(data) => {
                append_scale_constraint_lines(
                    data.constrained_bone,
                    data.target_bone,
                    &display_positions,
                    mesh,
                );
            }
            ConstraintType::Parent(data) => {
                append_parent_constraint_lines(
                    data.constrained_bone,
                    &data.sources,
                    &display_positions,
                    mesh,
                );
            }
            ConstraintType::Aim(data) => {
                append_aim_constraint_lines(
                    data.source_bone,
                    data.target_bone,
                    &display_positions,
                    mesh,
                );
            }
            ConstraintType::Ik(data) => {
                append_ik_constraint_lines(
                    data.effector_bone,
                    data.target_bone,
                    data.chain_length,
                    data.pole_target,
                    &display_positions,
                    skeleton,
                    mesh,
                );
            }
        }
    }
}

fn append_position_constraint_lines(
    constrained_bone: BoneId,
    target_bone: BoneId,
    display_positions: &[[f32; 3]],
    mesh: &mut LineMesh,
) {
    let (from, to) = match resolve_bone_positions(
        constrained_bone,
        target_bone,
        display_positions,
    ) {
        Some(pair) => pair,
        None => return,
    };

    append_dashed_line(mesh, from, to, COLOR_POSITION, DASH_SEGMENTS);
}

fn append_rotation_constraint_lines(
    constrained_bone: BoneId,
    target_bone: BoneId,
    display_positions: &[[f32; 3]],
    mesh: &mut LineMesh,
) {
    let (from, to) = match resolve_bone_positions(
        constrained_bone,
        target_bone,
        display_positions,
    ) {
        Some(pair) => pair,
        None => return,
    };

    append_dashed_line(mesh, from, to, COLOR_ROTATION, DASH_SEGMENTS);
}

fn append_scale_constraint_lines(
    constrained_bone: BoneId,
    target_bone: BoneId,
    display_positions: &[[f32; 3]],
    mesh: &mut LineMesh,
) {
    let (from, to) = match resolve_bone_positions(
        constrained_bone,
        target_bone,
        display_positions,
    ) {
        Some(pair) => pair,
        None => return,
    };

    append_dashed_line(mesh, from, to, COLOR_SCALE, DASH_SEGMENTS);
}

fn append_parent_constraint_lines(
    constrained_bone: BoneId,
    sources: &[(BoneId, f32)],
    display_positions: &[[f32; 3]],
    mesh: &mut LineMesh,
) {
    let constrained_idx = constrained_bone as usize;
    if constrained_idx >= display_positions.len() {
        return;
    }
    let from = display_positions[constrained_idx];

    for &(source_bone, _weight) in sources {
        let source_idx = source_bone as usize;
        if source_idx >= display_positions.len() {
            continue;
        }
        let to = display_positions[source_idx];
        append_dashed_line(mesh, from, to, COLOR_PARENT, DASH_SEGMENTS);
    }
}

fn append_aim_constraint_lines(
    source_bone: BoneId,
    target_bone: BoneId,
    display_positions: &[[f32; 3]],
    mesh: &mut LineMesh,
) {
    let (from, to) = match resolve_bone_positions(
        source_bone,
        target_bone,
        display_positions,
    ) {
        Some(pair) => pair,
        None => return,
    };

    append_solid_line(mesh, from, to, COLOR_AIM);
    append_arrow_head(mesh, from, to, COLOR_AIM, ARROW_SIZE);
}

fn append_ik_constraint_lines(
    effector_bone: BoneId,
    target_bone: BoneId,
    chain_length: u32,
    pole_target: Option<BoneId>,
    display_positions: &[[f32; 3]],
    skeleton: &Skeleton,
    mesh: &mut LineMesh,
) {
    let chain = collect_ik_chain(
        effector_bone,
        chain_length,
        skeleton,
    );

    for window in chain.windows(2) {
        let from_idx = window[0] as usize;
        let to_idx = window[1] as usize;
        if from_idx >= display_positions.len()
            || to_idx >= display_positions.len()
        {
            continue;
        }
        append_solid_line(
            mesh,
            display_positions[from_idx],
            display_positions[to_idx],
            COLOR_IK_CHAIN,
        );
    }

    let effector_idx = effector_bone as usize;
    let target_idx = target_bone as usize;
    if effector_idx < display_positions.len()
        && target_idx < display_positions.len()
    {
        append_dashed_line(
            mesh,
            display_positions[effector_idx],
            display_positions[target_idx],
            COLOR_IK_TARGET,
            DASH_SEGMENTS,
        );
    }

    if let Some(pole_bone) = pole_target {
        let mid_bone = find_mid_bone(&chain);
        if let Some(mid) = mid_bone {
            let mid_idx = mid as usize;
            let pole_idx = pole_bone as usize;
            if mid_idx < display_positions.len()
                && pole_idx < display_positions.len()
            {
                append_dashed_line(
                    mesh,
                    display_positions[mid_idx],
                    display_positions[pole_idx],
                    COLOR_IK_TARGET,
                    DASH_SEGMENTS / 2,
                );
            }
        }
    }
}

fn collect_ik_chain(
    effector_bone: BoneId,
    chain_length: u32,
    skeleton: &Skeleton,
) -> Vec<BoneId> {
    let mut chain = vec![effector_bone];
    let mut current = effector_bone;

    for _ in 0..chain_length {
        let idx = current as usize;
        if idx >= skeleton.bones.len() {
            break;
        }

        match skeleton.bones[idx].parent_id {
            Some(parent_id) => {
                chain.push(parent_id);
                current = parent_id;
            }
            None => break,
        }
    }

    chain
}

fn find_mid_bone(chain: &[BoneId]) -> Option<BoneId> {
    if chain.len() >= 3 {
        Some(chain[chain.len() / 2])
    } else if chain.len() == 2 {
        Some(chain[1])
    } else {
        None
    }
}

fn resolve_bone_positions(
    bone_a: BoneId,
    bone_b: BoneId,
    display_positions: &[[f32; 3]],
) -> Option<([f32; 3], [f32; 3])> {
    let idx_a = bone_a as usize;
    let idx_b = bone_b as usize;

    if idx_a >= display_positions.len()
        || idx_b >= display_positions.len()
    {
        return None;
    }

    Some((display_positions[idx_a], display_positions[idx_b]))
}

fn append_dashed_line(
    mesh: &mut LineMesh,
    from: [f32; 3],
    to: [f32; 3],
    color: [f32; 3],
    segment_count: u32,
) {
    let total = segment_count.max(2);

    for i in (0..total).step_by(2) {
        let t0 = i as f32 / total as f32;
        let t1 = ((i + 1) as f32 / total as f32).min(1.0);

        let p0 = lerp_pos(from, to, t0);
        let p1 = lerp_pos(from, to, t1);

        let base = mesh.vertices.len() as u32;
        mesh.vertices.push(ColorVertex { pos: p0, color });
        mesh.vertices.push(ColorVertex { pos: p1, color });
        mesh.indices.push(base);
        mesh.indices.push(base + 1);
    }
}

fn append_solid_line(
    mesh: &mut LineMesh,
    from: [f32; 3],
    to: [f32; 3],
    color: [f32; 3],
) {
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(ColorVertex { pos: from, color });
    mesh.vertices.push(ColorVertex { pos: to, color });
    mesh.indices.push(base);
    mesh.indices.push(base + 1);
}

fn append_arrow_head(
    mesh: &mut LineMesh,
    from: [f32; 3],
    to: [f32; 3],
    color: [f32; 3],
    size: f32,
) {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let dz = to[2] - from[2];
    let len = (dx * dx + dy * dy + dz * dz).sqrt();

    if len < 1e-6 {
        return;
    }

    let inv = 1.0 / len;
    let dir = [dx * inv, dy * inv, dz * inv];

    let up = if dir[1].abs() < 0.99 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };

    let right = normalize_vec(cross_vec(dir, up));
    let forward = normalize_vec(cross_vec(dir, right));

    let back = [
        to[0] - dir[0] * size,
        to[1] - dir[1] * size,
        to[2] - dir[2] * size,
    ];

    let wing_r = [
        back[0] + right[0] * size * 0.5,
        back[1] + right[1] * size * 0.5,
        back[2] + right[2] * size * 0.5,
    ];
    let wing_l = [
        back[0] - right[0] * size * 0.5,
        back[1] - right[1] * size * 0.5,
        back[2] - right[2] * size * 0.5,
    ];
    let wing_u = [
        back[0] + forward[0] * size * 0.5,
        back[1] + forward[1] * size * 0.5,
        back[2] + forward[2] * size * 0.5,
    ];
    let wing_d = [
        back[0] - forward[0] * size * 0.5,
        back[1] - forward[1] * size * 0.5,
        back[2] - forward[2] * size * 0.5,
    ];

    append_solid_line(mesh, to, wing_r, color);
    append_solid_line(mesh, to, wing_l, color);
    append_solid_line(mesh, to, wing_u, color);
    append_solid_line(mesh, to, wing_d, color);
}

fn lerp_pos(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn cross_vec(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize_vec(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-10 {
        return [0.0, 0.0, 0.0];
    }
    let inv = 1.0 / len;
    [v[0] * inv, v[1] * inv, v[2] * inv]
}
