use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3};

use crate::animation::spring_bone::{
    apply_length_constraint, compute_joint_rotation, compute_tail_position, conjugate,
    extract_world_position, integrate_joint, recompute_global_transform,
    resolve_all_segment_collisions, resolve_all_tail_collisions, rotate_vector,
    segment_axis_distance, tail_axis_distance, JointForces, JointRestFrame, SpringCollisionMode,
    SpringForceModel, SpringSpace, WorldCollider, EARTH_GRAVITY_M_PER_S2,
};
use crate::animation::{compose_transform, decompose_transform, BoneId, Skeleton, SkeletonPose};
use crate::asset::AssetStorage;
use crate::ecs::component::{ColliderShape, SpringBoneSetup, WithSpringBone};
use crate::ecs::resource::{SpringBoneState, SpringChainState, SpringJointState};
use crate::ecs::world::{Animator, Entity, Parent, Transform, World};
use crate::hooks::model_load::LoadedModel;

const REST_VELOCITY_WORLD_UNITS: f32 = 1e-4;

/// The uniform scale the entity's ancestry applies to its skeleton, as the space the spring integrates in.
pub fn spring_space_for_entity(world: &World, entity: Entity) -> SpringSpace {
    let mut world_scale = 1.0;
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Some(transform) = world.get_component::<Transform>(e) {
            world_scale *= (transform.scale.x + transform.scale.y + transform.scale.z) / 3.0;
        }
        current = world.get_component::<Parent>(e).map(|p| p.0);
    }
    SpringSpace::from_world_scale(world_scale).unwrap_or(SpringSpace::IDENTITY)
}

pub fn collider_shape_radius(shape: &ColliderShape) -> f32 {
    match shape {
        ColliderShape::Sphere { radius } => *radius,
        ColliderShape::Capsule { radius, .. } => *radius,
    }
}

pub fn spring_bone_initialize(
    setup: &SpringBoneSetup,
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
) -> SpringBoneState {
    log!("[SpringBone] Initializing spring bone state...");
    let mut chain_states = Vec::new();

    for chain in &setup.chains {
        if !chain.enabled {
            log!(
                "[SpringBone]   Chain '{}' is disabled, skipping",
                chain.name
            );
            continue;
        }

        log!(
            "[SpringBone]   Chain '{}' (id={}, joints={})",
            chain.name,
            chain.id,
            chain.joints.len()
        );

        let world_colliders =
            collect_world_colliders(setup, &chain.collider_group_ids, global_transforms);
        let mut joint_states = Vec::new();

        for joint_param in &chain.joints {
            let bone_id = joint_param.bone_id;
            let idx = bone_id as usize;

            if idx >= skeleton.bones.len() || idx >= global_transforms.len() {
                log!(
                    "[SpringBone]     bone_id={} out of range \
                     (bones={}, transforms={})",
                    bone_id,
                    skeleton.bones.len(),
                    global_transforms.len()
                );
                continue;
            }

            let bone = &skeleton.bones[idx];
            let (_, initial_local_rotation, _) = decompose_transform(&bone.local_transform);

            let (bone_length, bone_axis) =
                compute_bone_length_and_axis(skeleton, bone_id, global_transforms);

            let head_pos = extract_world_position(&global_transforms[idx]);

            let tail = compute_initial_tail_position(
                skeleton,
                bone_id,
                global_transforms,
                initial_local_rotation,
                bone_axis,
                bone_length,
            );

            log!(
                "[SpringBone]     Joint '{}' (bone_id={}): \
                 head=({:.3},{:.3},{:.3}), \
                 tail=({:.3},{:.3},{:.3}), \
                 length={:.4}, axis=({:.3},{:.3},{:.3})",
                bone.name,
                bone_id,
                head_pos.x,
                head_pos.y,
                head_pos.z,
                tail.x,
                tail.y,
                tail.z,
                bone_length,
                bone_axis.x,
                bone_axis.y,
                bone_axis.z,
            );

            let rest_axis_distances =
                rest_axis_distances(head_pos, tail, &world_colliders, setup.collision_mode);
            let parent_world_rotation =
                extract_parent_world_rotation(skeleton, idx, global_transforms);
            let gravity_in_parent_rest =
                rotate_vector(conjugate(parent_world_rotation), joint_param.gravity_dir);

            joint_states.push(SpringJointState {
                prev_tail: tail,
                current_tail: tail,
                bone_length,
                bone_axis,
                initial_local_rotation,
                rest_axis_distances,
                gravity_in_parent_rest,
            });
        }

        chain_states.push(SpringChainState {
            chain_id: chain.id,
            joint_states,
        });
    }

    log!(
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
    pose: &SkeletonPose,
    dt: f32,
    space: SpringSpace,
) {
    let dt = dt.min(state.max_delta_time);
    if dt <= 0.0 {
        return;
    }

    let should_log = state.frame_count < state.log_frames;
    if should_log {
        log!(
            "[SpringBone] Update frame={}, dt={:.4}",
            state.frame_count,
            dt
        );
    }

    let enabled_chains: Vec<_> = setup.chains.iter().filter(|c| c.enabled).collect();

    for (state_idx, chain) in enabled_chains.iter().enumerate() {
        if state_idx >= state.chain_states.len() {
            break;
        }

        let world_colliders =
            collect_world_colliders(setup, &chain.collider_group_ids, global_transforms);

        let chain_state = &mut state.chain_states[state_idx];

        for (joint_idx, joint_param) in chain.joints.iter().enumerate() {
            if joint_idx >= chain_state.joint_states.len() {
                break;
            }

            update_single_joint(
                joint_param,
                &mut chain_state.joint_states[joint_idx],
                skeleton,
                global_transforms,
                pose,
                &world_colliders,
                should_log,
                dt,
                space,
                setup.collision_mode,
                setup.force_model,
            );
        }
    }

    state.frame_count += 1;
}

fn update_single_joint(
    joint_param: &crate::ecs::component::SpringJointParam,
    joint_state: &mut SpringJointState,
    skeleton: &Skeleton,
    global_transforms: &mut [Matrix4<f32>],
    pose: &SkeletonPose,
    world_colliders: &[WorldCollider],
    should_log: bool,
    dt: f32,
    space: SpringSpace,
    collision_mode: SpringCollisionMode,
    force_model: SpringForceModel,
) {
    let bone_id = joint_param.bone_id;
    let idx = bone_id as usize;
    if idx >= skeleton.bones.len() || idx >= global_transforms.len() {
        return;
    }

    let head_pos = extract_world_position(&global_transforms[idx]);
    let parent_world_rot = extract_parent_world_rotation(skeleton, idx, global_transforms);

    let current_tail_before = joint_state.current_tail;

    let hit_radius = space.skeleton_length(joint_param.hit_radius);
    let rest_frame = JointRestFrame {
        parent_world_rotation: parent_world_rot,
        initial_local_rotation: joint_state.initial_local_rotation,
        bone_axis: joint_state.bone_axis,
        bone_length: joint_state.bone_length,
        gravity_in_parent_rest: joint_state.gravity_in_parent_rest,
    };
    let forces = joint_forces(joint_param, space, force_model);

    let next_tail = integrate_joint(
        joint_state.current_tail,
        joint_state.prev_tail,
        &rest_frame,
        &forces,
        force_model,
        dt,
    );

    let constrained_tail = apply_length_constraint(head_pos, next_tail, joint_state.bone_length);

    let collision_resolved = match collision_mode {
        SpringCollisionMode::VrmCompatible => resolve_all_tail_collisions(
            constrained_tail,
            hit_radius,
            world_colliders,
            &joint_state.rest_axis_distances,
        ),
        SpringCollisionMode::SegmentExtended => resolve_all_segment_collisions(
            head_pos,
            constrained_tail,
            hit_radius,
            world_colliders,
            &joint_state.rest_axis_distances,
        ),
    };
    let had_collision = !approx_eq_vec3(collision_resolved, constrained_tail);
    let resolved_tail =
        apply_length_constraint(head_pos, collision_resolved, joint_state.bone_length);

    if should_log {
        log_joint_update(
            skeleton,
            bone_id,
            head_pos,
            joint_state,
            next_tail,
            resolved_tail,
            current_tail_before,
            had_collision,
        );
    }

    let velocity = resolved_tail - joint_state.current_tail;
    let velocity_mag_sq =
        velocity.x * velocity.x + velocity.y * velocity.y + velocity.z * velocity.z;

    let rest_velocity = space.skeleton_length(REST_VELOCITY_WORLD_UNITS);
    if velocity_mag_sq < rest_velocity * rest_velocity {
        joint_state.prev_tail = resolved_tail;
        joint_state.current_tail = resolved_tail;
    } else {
        joint_state.prev_tail = joint_state.current_tail;
        joint_state.current_tail = resolved_tail;
    }

    let new_local_rotation = compute_joint_rotation(
        head_pos,
        parent_world_rot,
        joint_state.initial_local_rotation,
        joint_state.bone_axis,
        resolved_tail,
    );

    update_global_transforms_for_bone(
        skeleton,
        bone_id,
        new_local_rotation,
        global_transforms,
        pose,
    );
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

/// VRM reads `stiffness` and `gravity_power` as metres per second; the rest-relative model reads
/// `stiffness` as a rate and `gravity_power` as a multiple of Earth gravity.
fn joint_forces(
    joint_param: &crate::ecs::component::SpringJointParam,
    space: SpringSpace,
    force_model: SpringForceModel,
) -> JointForces {
    let (stiffness, gravity) = match force_model {
        SpringForceModel::Vrm => (
            space.skeleton_length(joint_param.stiffness),
            space.skeleton_length(joint_param.gravity_power),
        ),
        SpringForceModel::RestRelative => (
            joint_param.stiffness,
            space.skeleton_length(joint_param.gravity_power * EARTH_GRAVITY_M_PER_S2),
        ),
    };
    JointForces {
        drag_force: joint_param.drag_force,
        stiffness,
        gravity_dir: joint_param.gravity_dir,
        gravity,
    }
}

fn rest_axis_distances(
    head: Vector3<f32>,
    tail: Vector3<f32>,
    world_colliders: &[WorldCollider],
    collision_mode: SpringCollisionMode,
) -> Vec<f32> {
    world_colliders
        .iter()
        .map(|collider| match collision_mode {
            SpringCollisionMode::VrmCompatible => tail_axis_distance(tail, collider),
            SpringCollisionMode::SegmentExtended => segment_axis_distance(head, tail, collider),
        })
        .collect()
}

fn compute_initial_tail_position(
    skeleton: &Skeleton,
    bone_id: BoneId,
    global_transforms: &[Matrix4<f32>],
    initial_local_rotation: Quaternion<f32>,
    bone_axis: Vector3<f32>,
    bone_length: f32,
) -> Vector3<f32> {
    let idx = bone_id as usize;

    if let Some(&child_id) = skeleton.bones[idx].children.first() {
        let cidx = child_id as usize;
        if cidx < global_transforms.len() {
            return extract_world_position(&global_transforms[cidx]);
        }
    }

    let head_pos = extract_world_position(&global_transforms[idx]);
    let parent_rot = extract_parent_world_rotation(skeleton, idx, global_transforms);
    compute_tail_position(
        head_pos,
        parent_rot,
        initial_local_rotation,
        bone_axis,
        bone_length,
    )
}

fn extract_parent_world_rotation(
    skeleton: &Skeleton,
    bone_idx: usize,
    global_transforms: &[Matrix4<f32>],
) -> Quaternion<f32> {
    match skeleton.bones[bone_idx].parent_id {
        Some(parent_id) => {
            let pidx = parent_id as usize;
            if pidx < global_transforms.len() {
                let (_, r, _) = decompose_transform(&global_transforms[pidx]);
                r
            } else {
                Quaternion::new(1.0, 0.0, 0.0, 0.0)
            }
        }
        None => {
            let (_, r, _) = decompose_transform(&skeleton.root_transform);
            r
        }
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
                let (_, bone_rot, _) = decompose_transform(&global_transforms[idx]);
                let inv_q = cgmath::Quaternion::new(
                    bone_rot.s,
                    -bone_rot.v.x,
                    -bone_rot.v.y,
                    -bone_rot.v.z,
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

fn rotate_vec3_by_quat(q: cgmath::Quaternion<f32>, v: Vector3<f32>) -> Vector3<f32> {
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
    pose: &SkeletonPose,
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
        recompute_children_globals(skeleton, child_id, global_transforms, pose);
    }
}

fn recompute_children_globals(
    skeleton: &Skeleton,
    bone_id: BoneId,
    global_transforms: &mut [Matrix4<f32>],
    pose: &SkeletonPose,
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

    let animated_local = if idx < pose.bone_poses.len() {
        compose_transform(
            pose.bone_poses[idx].translation,
            pose.bone_poses[idx].rotation,
            pose.bone_poses[idx].scale,
        )
    } else {
        skeleton.bones[idx].local_transform
    };

    global_transforms[idx] = parent_global * animated_local;

    for &child_id in &skeleton.bones[idx].children {
        recompute_children_globals(skeleton, child_id, global_transforms, pose);
    }
}

fn transform_point(matrix: &Matrix4<f32>, offset: Vector3<f32>) -> Vector3<f32> {
    let x =
        matrix[0][0] * offset.x + matrix[1][0] * offset.y + matrix[2][0] * offset.z + matrix[3][0];
    let y =
        matrix[0][1] * offset.x + matrix[1][1] * offset.y + matrix[2][1] * offset.z + matrix[3][1];
    let z =
        matrix[0][2] * offset.x + matrix[1][2] * offset.y + matrix[2][2] * offset.z + matrix[3][2];
    Vector3::new(x, y, z)
}

fn collect_world_colliders(
    setup: &SpringBoneSetup,
    collider_group_ids: &[u32],
    global_transforms: &[Matrix4<f32>],
) -> Vec<WorldCollider> {
    let mut world_colliders = Vec::new();

    for &group_id in collider_group_ids {
        let group = setup.collider_groups.iter().find(|g| g.id == group_id);
        let Some(group) = group else { continue };

        for &collider_id in &group.collider_ids {
            let collider_def = setup.colliders.iter().find(|c| c.id == collider_id);
            let Some(def) = collider_def else { continue };

            let bone_idx = def.bone_id as usize;
            if bone_idx >= global_transforms.len() {
                continue;
            }

            let bone_transform = &global_transforms[bone_idx];
            let center = transform_point(bone_transform, def.offset);

            match &def.shape {
                ColliderShape::Sphere { .. } => {
                    world_colliders.push(WorldCollider {
                        center,
                        radius: collider_shape_radius(&def.shape),
                        tail: None,
                    });
                }
                ColliderShape::Capsule { tail, .. } => {
                    let world_tail = transform_point(bone_transform, *tail);
                    world_colliders.push(WorldCollider {
                        center,
                        radius: collider_shape_radius(&def.shape),
                        tail: Some(world_tail),
                    });
                }
            }
        }
    }

    world_colliders
}

fn approx_eq_vec3(a: Vector3<f32>, b: Vector3<f32>) -> bool {
    let d = a - b;
    (d.x * d.x + d.y * d.y + d.z * d.z) < 1e-12
}

pub fn collect_affected_bone_ids(setup: &SpringBoneSetup) -> Vec<BoneId> {
    let mut ids: Vec<BoneId> = setup
        .chains
        .iter()
        .filter(|c| c.enabled)
        .flat_map(|c| c.joints.iter().map(|j| j.bone_id))
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

fn log_joint_update(
    skeleton: &Skeleton,
    bone_id: BoneId,
    head_pos: Vector3<f32>,
    joint_state: &SpringJointState,
    next_tail: Vector3<f32>,
    resolved_tail: Vector3<f32>,
    current_tail_before: Vector3<f32>,
    had_collision: bool,
) {
    let bone_name = skeleton
        .get_bone(bone_id)
        .map(|b| b.name.as_str())
        .unwrap_or("?");
    let movement = resolved_tail - current_tail_before;
    let move_mag =
        (movement.x * movement.x + movement.y * movement.y + movement.z * movement.z).sqrt();
    let collision_marker = if had_collision { " [HIT]" } else { "" };
    log!(
        "[SpringBone]   Joint '{}'{}: \
         head=({:.3},{:.3},{:.3}), \
         prev_tail=({:.3},{:.3},{:.3}), \
         cur_tail=({:.3},{:.3},{:.3}), \
         next=({:.3},{:.3},{:.3}), \
         resolved=({:.3},{:.3},{:.3}), \
         delta={:.6}",
        bone_name,
        collision_marker,
        head_pos.x,
        head_pos.y,
        head_pos.z,
        joint_state.prev_tail.x,
        joint_state.prev_tail.y,
        joint_state.prev_tail.z,
        current_tail_before.x,
        current_tail_before.y,
        current_tail_before.z,
        next_tail.x,
        next_tail.y,
        next_tail.z,
        resolved_tail.x,
        resolved_tail.y,
        resolved_tail.z,
        move_mag,
    );
}

pub fn spring_bone_attach_loaded(world: &mut World, _assets: &AssetStorage, loaded: &LoadedModel) {
    let Some(setup) = loaded.load_result.spring_bone_setup.as_ref() else {
        return;
    };
    if !world.has_component::<Animator>(loaded.entity) {
        return;
    }

    world.insert_component(loaded.entity, setup.clone());
    world.insert_component(loaded.entity, WithSpringBone);

    log!(
        "Applied VRMC spring bones to entity {}: {} chains, {} colliders",
        loaded.entity,
        setup.chains.len(),
        setup.colliders.len()
    );
}

crate::model_load_hook!("spring_bone", Rig, spring_bone_attach_loaded);
