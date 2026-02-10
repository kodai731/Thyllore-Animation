use cgmath::{Matrix4, SquareMatrix, Vector3};

use crate::animation::spring_bone::{
    apply_length_constraint, compute_joint_rotation, compute_tail_position,
    extract_world_position, integrate_joint, recompute_global_transform,
};
use crate::animation::{
    compose_transform, decompose_transform, BoneId, Skeleton,
    SkeletonPose,
};
use crate::ecs::component::SpringBoneSetup;
use crate::ecs::resource::{
    SpringBoneState, SpringChainState, SpringJointState,
};

pub fn spring_bone_initialize(
    setup: &SpringBoneSetup,
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
) -> SpringBoneState {
    crate::log!("[SpringBone] Initializing spring bone state...");
    let mut chain_states = Vec::new();

    for chain in &setup.chains {
        if !chain.enabled {
            crate::log!("[SpringBone]   Chain '{}' is disabled, skipping", chain.name);
            continue;
        }

        crate::log!(
            "[SpringBone]   Chain '{}' (id={}, joints={})",
            chain.name,
            chain.id,
            chain.joints.len()
        );

        let mut joint_states = Vec::new();

        for joint_param in &chain.joints {
            let bone_id = joint_param.bone_id;
            let idx = bone_id as usize;

            if idx >= skeleton.bones.len() || idx >= global_transforms.len() {
                crate::log!(
                    "[SpringBone]     bone_id={} out of range (bones={}, transforms={})",
                    bone_id,
                    skeleton.bones.len(),
                    global_transforms.len()
                );
                continue;
            }

            let bone = &skeleton.bones[idx];
            let (_, initial_local_rotation, _) =
                decompose_transform(&bone.local_transform);

            let (bone_length, bone_axis) =
                compute_bone_length_and_axis(skeleton, bone_id, global_transforms);

            let tail = compute_tail_position(
                &global_transforms[idx],
                initial_local_rotation,
                bone_axis,
                bone_length,
            );

            let head_pos = extract_world_position(&global_transforms[idx]);
            crate::log!(
                "[SpringBone]     Joint '{}' (bone_id={}): head=({:.3},{:.3},{:.3}), \
                 tail=({:.3},{:.3},{:.3}), length={:.4}, axis=({:.3},{:.3},{:.3})",
                bone.name,
                bone_id,
                head_pos.x, head_pos.y, head_pos.z,
                tail.x, tail.y, tail.z,
                bone_length,
                bone_axis.x, bone_axis.y, bone_axis.z,
            );

            joint_states.push(SpringJointState {
                prev_tail: tail,
                current_tail: tail,
                bone_length,
                bone_axis,
                initial_local_rotation,
            });
        }

        chain_states.push(SpringChainState {
            chain_id: chain.id,
            joint_states,
        });
    }

    crate::log!(
        "[SpringBone] Initialization complete: {} chain states",
        chain_states.len()
    );

    SpringBoneState {
        chain_states,
        initialized: true,
        ..Default::default()
    }
}

pub fn spring_bone_update(
    setup: &SpringBoneSetup,
    state: &mut SpringBoneState,
    skeleton: &Skeleton,
    global_transforms: &mut [Matrix4<f32>],
    dt: f32,
) {
    let dt = dt.min(state.max_delta_time);
    if dt <= 0.0 {
        return;
    }

    let should_log = state.frame_count < state.log_frames;
    if should_log {
        crate::log!(
            "[SpringBone] Update frame={}, dt={:.4}",
            state.frame_count,
            dt
        );
    }

    let enabled_chains: Vec<_> = setup
        .chains
        .iter()
        .filter(|c| c.enabled)
        .collect();

    for (state_idx, chain) in enabled_chains.iter().enumerate() {
        if state_idx >= state.chain_states.len() {
            break;
        }

        let chain_state = &mut state.chain_states[state_idx];

        for (joint_idx, joint_param) in chain.joints.iter().enumerate() {
            if joint_idx >= chain_state.joint_states.len() {
                break;
            }

            let bone_id = joint_param.bone_id;
            let idx = bone_id as usize;
            if idx >= skeleton.bones.len() || idx >= global_transforms.len() {
                continue;
            }

            let head_pos = extract_world_position(&global_transforms[idx]);
            let (_, parent_world_rot, _) =
                decompose_transform(&global_transforms[idx]);

            let joint_state = &chain_state.joint_states[joint_idx];
            let prev_tail_before = joint_state.prev_tail;
            let current_tail_before = joint_state.current_tail;

            let next_tail = integrate_joint(
                joint_state.current_tail,
                joint_state.prev_tail,
                joint_param.drag_force,
                joint_param.stiffness,
                joint_param.gravity_dir,
                joint_param.gravity_power,
                parent_world_rot,
                joint_state.initial_local_rotation,
                joint_state.bone_axis,
                joint_state.bone_length,
                head_pos,
                dt,
            );

            let constrained_tail =
                apply_length_constraint(head_pos, next_tail, joint_state.bone_length);

            if should_log {
                let bone_name = skeleton
                    .get_bone(bone_id)
                    .map(|b| b.name.as_str())
                    .unwrap_or("?");
                let movement = Vector3::new(
                    constrained_tail.x - current_tail_before.x,
                    constrained_tail.y - current_tail_before.y,
                    constrained_tail.z - current_tail_before.z,
                );
                let move_mag = (movement.x * movement.x
                    + movement.y * movement.y
                    + movement.z * movement.z)
                    .sqrt();
                crate::log!(
                    "[SpringBone]   Joint '{}': head=({:.3},{:.3},{:.3}), \
                     prev_tail=({:.3},{:.3},{:.3}), cur_tail=({:.3},{:.3},{:.3}), \
                     next=({:.3},{:.3},{:.3}), constrained=({:.3},{:.3},{:.3}), \
                     delta={:.6}",
                    bone_name,
                    head_pos.x, head_pos.y, head_pos.z,
                    prev_tail_before.x, prev_tail_before.y, prev_tail_before.z,
                    current_tail_before.x, current_tail_before.y, current_tail_before.z,
                    next_tail.x, next_tail.y, next_tail.z,
                    constrained_tail.x, constrained_tail.y, constrained_tail.z,
                    move_mag,
                );
            }

            let joint_state = &mut chain_state.joint_states[joint_idx];

            let velocity = constrained_tail - joint_state.current_tail;
            let velocity_mag_sq = velocity.x * velocity.x
                + velocity.y * velocity.y
                + velocity.z * velocity.z;

            const VELOCITY_THRESHOLD_SQ: f32 = 1e-8;
            if velocity_mag_sq < VELOCITY_THRESHOLD_SQ {
                joint_state.prev_tail = constrained_tail;
                joint_state.current_tail = constrained_tail;
            } else {
                joint_state.prev_tail = joint_state.current_tail;
                joint_state.current_tail = constrained_tail;
            }

            let new_local_rotation = compute_joint_rotation(
                global_transforms[idx],
                joint_state.initial_local_rotation,
                joint_state.bone_axis,
                constrained_tail,
            );

            update_global_transforms_for_bone(
                skeleton,
                bone_id,
                new_local_rotation,
                global_transforms,
            );
        }
    }

    state.frame_count += 1;
}

pub fn spring_bone_write_back_to_pose(
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    pose: &mut SkeletonPose,
    affected_bone_ids: &[BoneId],
) {
    for &bone_id in affected_bone_ids {
        let idx = bone_id as usize;
        if idx >= skeleton.bones.len()
            || idx >= global_transforms.len()
            || idx >= pose.bone_poses.len()
        {
            continue;
        }

        let parent_global = match skeleton.bones[idx].parent_id {
            Some(parent_id) => {
                let pidx = parent_id as usize;
                if pidx < global_transforms.len() {
                    global_transforms[pidx]
                } else {
                    skeleton.root_transform
                }
            }
            None => skeleton.root_transform,
        };

        let inv_parent = match parent_global.invert() {
            Some(inv) => inv,
            None => Matrix4::identity(),
        };

        let local = inv_parent * global_transforms[idx];
        let (t, r, s) = decompose_transform(&local);

        pose.bone_poses[idx].translation = t;
        pose.bone_poses[idx].rotation = r;
        pose.bone_poses[idx].scale = s;
    }
}

fn compute_bone_length_and_axis(
    skeleton: &Skeleton,
    bone_id: BoneId,
    global_transforms: &[Matrix4<f32>],
) -> (f32, Vector3<f32>) {
    let idx = bone_id as usize;
    let bone = &skeleton.bones[idx];

    if let Some(&child_id) = bone.children.first() {
        let cidx = child_id as usize;
        if cidx < global_transforms.len() {
            let parent_pos = extract_world_position(&global_transforms[idx]);
            let child_pos = extract_world_position(&global_transforms[cidx]);
            let diff = child_pos - parent_pos;
            let length = (diff.x * diff.x + diff.y * diff.y + diff.z * diff.z).sqrt();

            if length > 1e-6 {
                let (_, inv_rot, _) = decompose_transform(&global_transforms[idx]);
                let inv_q = cgmath::Quaternion::new(
                    inv_rot.s,
                    -inv_rot.v.x,
                    -inv_rot.v.y,
                    -inv_rot.v.z,
                );
                let local_dir = rotate_vec3_by_quat(inv_q, diff / length);
                return (length, local_dir);
            }
        }
    }

    let (t, _, _) = decompose_transform(&bone.local_transform);
    let length = (t.x * t.x + t.y * t.y + t.z * t.z).sqrt();

    if length > 1e-6 {
        (length, t / length)
    } else {
        (0.1, Vector3::new(0.0, 1.0, 0.0))
    }
}

fn rotate_vec3_by_quat(
    q: cgmath::Quaternion<f32>,
    v: Vector3<f32>,
) -> Vector3<f32> {
    let qv = cgmath::Quaternion::new(0.0, v.x, v.y, v.z);
    let conj = cgmath::Quaternion::new(q.s, -q.v.x, -q.v.y, -q.v.z);
    let result = q * qv * conj;
    Vector3::new(result.v.x, result.v.y, result.v.z)
}

fn update_global_transforms_for_bone(
    skeleton: &Skeleton,
    bone_id: BoneId,
    new_local_rotation: cgmath::Quaternion<f32>,
    global_transforms: &mut [Matrix4<f32>],
) {
    let idx = bone_id as usize;
    if idx >= skeleton.bones.len() {
        return;
    }

    let parent_global = match skeleton.bones[idx].parent_id {
        Some(parent_id) => {
            let pidx = parent_id as usize;
            if pidx < global_transforms.len() {
                global_transforms[pidx]
            } else {
                Matrix4::identity()
            }
        }
        None => skeleton.root_transform,
    };

    let new_global = recompute_global_transform(
        &parent_global,
        new_local_rotation,
        &skeleton.bones[idx].local_transform,
    );
    global_transforms[idx] = new_global;

    for &child_id in &skeleton.bones[idx].children {
        recompute_children_globals(skeleton, child_id, global_transforms);
    }
}

fn recompute_children_globals(
    skeleton: &Skeleton,
    bone_id: BoneId,
    global_transforms: &mut [Matrix4<f32>],
) {
    let idx = bone_id as usize;
    if idx >= skeleton.bones.len() || idx >= global_transforms.len() {
        return;
    }

    let parent_global = match skeleton.bones[idx].parent_id {
        Some(parent_id) => {
            let pidx = parent_id as usize;
            if pidx < global_transforms.len() {
                global_transforms[pidx]
            } else {
                Matrix4::identity()
            }
        }
        None => skeleton.root_transform,
    };

    global_transforms[idx] = parent_global * skeleton.bones[idx].local_transform;

    for &child_id in &skeleton.bones[idx].children {
        recompute_children_globals(skeleton, child_id, global_transforms);
    }
}
