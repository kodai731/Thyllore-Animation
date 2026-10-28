use cgmath::{InnerSpace, Matrix3, Matrix4, Quaternion, SquareMatrix, Vector3};

use crate::animation::spring_bone::{
    apply_length_constraint, compute_joint_rotation, compute_tail_position, conjugate,
    extract_world_position, integrate_joint, recompute_global_transform,
    resolve_all_segment_collisions, resolve_all_tail_collisions, rotate_vector,
    segment_axis_distance, tail_axis_distance, JointForce, JointForces, JointRestFrame,
    SpringCollisionMode, SpringSpace, WorldCollider, EARTH_GRAVITY_M_PER_S2,
};
use crate::animation::{compose_transform, decompose_transform, BoneId, Skeleton, SkeletonPose};
use crate::asset::AssetStorage;
use crate::ecs::component::{ColliderShape, SpringBoneSetup, WithSpringBone};
use crate::ecs::resource::{SpringBoneState, SpringChainState, SpringJointState};
use crate::ecs::world::{Animator, Entity, Parent, Transform, World};
use crate::ecs::MeshRef;
use crate::hooks::model_load::LoadedModel;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use thyllore_anim_core::spring_bone::{chains_skinned_to, SpringChainId};
use thyllore_math_core::mean_axis_scale;

const REST_VELOCITY_WORLD_UNITS: f32 = 1e-4;

/// The uniform scale the entity's ancestry applies to its skeleton; a mirrored axis counts by its magnitude.
pub fn spring_space_for_entity(world: &World, entity: Entity) -> SpringSpace {
    let mut world_scale = 1.0;
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Some(transform) = world.get_component::<Transform>(e) {
            let scale = transform.scale;
            world_scale *= (scale.x.abs() + scale.y.abs() + scale.z.abs()) / 3.0;
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

        for (joint_index, joint_param) in chain.joints.iter().enumerate() {
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

            let tail_bone = joint_tail_bone(chain, joint_index, skeleton);
            let (bone_length, bone_axis) =
                compute_bone_length_and_axis(skeleton, bone_id, tail_bone, global_transforms);

            let head_pos = extract_world_position(&global_transforms[idx]);

            let tail = compute_initial_tail_position(
                skeleton,
                bone_id,
                tail_bone,
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

            let rest_axis_distances = match joint_param.force {
                JointForce::RestRelative(_) => {
                    rest_axis_distances(head_pos, tail, &world_colliders, setup.collision_mode)
                }
                JointForce::Vrm(_) => Vec::new(),
            };
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

        let strand_length = joint_states.iter().map(|joint| joint.bone_length).sum();
        chain_states.push(SpringChainState {
            chain_id: chain.id,
            joint_states,
            strand_length,
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
        let strand_length = chain_state.strand_length;

        for (joint_idx, joint_param) in chain.joints.iter().enumerate() {
            if joint_idx >= chain_state.joint_states.len() {
                break;
            }

            update_single_joint(
                joint_param,
                strand_length,
                &mut chain_state.joint_states[joint_idx],
                skeleton,
                global_transforms,
                pose,
                &world_colliders,
                should_log,
                dt,
                space,
                setup.collision_mode,
            );
        }
    }

    state.frame_count += 1;
}

fn update_single_joint(
    joint_param: &crate::ecs::component::SpringJointParam,
    strand_length: f32,
    joint_state: &mut SpringJointState,
    skeleton: &Skeleton,
    global_transforms: &mut [Matrix4<f32>],
    pose: &SkeletonPose,
    world_colliders: &[WorldCollider],
    should_log: bool,
    dt: f32,
    space: SpringSpace,
    collision_mode: SpringCollisionMode,
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
        head: head_pos,
        parent_world_rotation: parent_world_rot,
        initial_local_rotation: joint_state.initial_local_rotation,
        bone_axis: joint_state.bone_axis,
        bone_length: joint_state.bone_length,
        gravity_in_parent_rest: joint_state.gravity_in_parent_rest,
    };
    let forces = joint_forces(joint_param, strand_length, joint_state.bone_length, space);

    let next_tail = integrate_joint(
        joint_state.current_tail,
        joint_state.prev_tail,
        &rest_frame,
        &forces,
        joint_param.force.model(),
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

fn joint_forces(
    joint_param: &crate::ecs::component::SpringJointParam,
    strand_length: f32,
    bone_length: f32,
    space: SpringSpace,
) -> JointForces {
    let (stiffness, gravity) = match joint_param.force {
        JointForce::Vrm(force) => (
            space.skeleton_length(force.stiffness),
            space.skeleton_length(force.gravity_power),
        ),
        JointForce::RestRelative(force) => (
            strand_return_rate(
                space.skeleton_length(space.skeleton_length(force.bending_relaxation)),
                strand_length,
                bone_length,
            ),
            space.skeleton_length(force.gravity_scale * EARTH_GRAVITY_M_PER_S2),
        ),
    };
    JointForces {
        drag_force: joint_param.drag_force,
        stiffness,
        gravity_dir: joint_param.gravity_dir,
        gravity,
    }
}

/// A strand returns at `D / L²`; splitting it into bones of length `l` scales each joint's rate by `L / l`
/// so the strand's bending stiffness does not depend on the subdivision.
fn strand_return_rate(relaxation: f32, strand_length: f32, bone_length: f32) -> f32 {
    let lever = strand_length * bone_length;
    if lever > 0.0 {
        relaxation / lever
    } else {
        0.0
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

/// The bone a joint swings towards: the next joint, the declared tail, or the first child.
fn joint_tail_bone(
    chain: &crate::ecs::component::SpringChain,
    joint_index: usize,
    skeleton: &Skeleton,
) -> Option<BoneId> {
    let declared = match chain.joints.get(joint_index + 1) {
        Some(next) => Some(next.bone_id),
        None => chain.tail_bone_id,
    };
    declared.or_else(|| {
        skeleton
            .bones
            .get(chain.joints[joint_index].bone_id as usize)?
            .children
            .first()
            .copied()
    })
}

/// Moves every tail to where the bones point in `global_transforms` without touching the rest data.
pub fn spring_bone_reseat_tails(
    setup: &SpringBoneSetup,
    state: &mut SpringBoneState,
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
) {
    let enabled_chains = setup.chains.iter().filter(|chain| chain.enabled);
    for (chain, chain_state) in enabled_chains.zip(&mut state.chain_states) {
        let joints = chain.joints.iter().enumerate();
        for ((joint_index, joint_param), joint_state) in joints.zip(&mut chain_state.joint_states) {
            if joint_param.bone_id as usize >= global_transforms.len() {
                continue;
            }
            let tail = compute_initial_tail_position(
                skeleton,
                joint_param.bone_id,
                joint_tail_bone(chain, joint_index, skeleton),
                global_transforms,
                joint_state.initial_local_rotation,
                joint_state.bone_axis,
                joint_state.bone_length,
            );
            joint_state.prev_tail = tail;
            joint_state.current_tail = tail;
        }
    }
}

fn compute_initial_tail_position(
    skeleton: &Skeleton,
    bone_id: BoneId,
    tail_bone: Option<BoneId>,
    global_transforms: &[Matrix4<f32>],
    initial_local_rotation: Quaternion<f32>,
    bone_axis: Vector3<f32>,
    bone_length: f32,
) -> Vector3<f32> {
    let idx = bone_id as usize;

    if let Some(tail_transform) = tail_bone.and_then(|id| global_transforms.get(id as usize)) {
        return extract_world_position(tail_transform);
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
    tail_bone: Option<BoneId>,
    global_transforms: &[Matrix4<f32>],
) -> (f32, Vector3<f32>) {
    let idx = bone_id as usize;

    if let Some(tail_transform) = tail_bone.and_then(|id| global_transforms.get(id as usize)) {
        let head_pos = extract_world_position(&global_transforms[idx]);
        let tail_pos = extract_world_position(tail_transform);
        let diff = tail_pos - head_pos;
        let length = diff.magnitude();

        if length > 1e-6 {
            let (_, bone_rot, _) = decompose_transform(&global_transforms[idx]);
            let local_dir = rotate_vec3_by_quat(conjugate(bone_rot), diff / length);
            return (length, local_dir);
        }
    }

    leaf_bone_length_and_axis(skeleton, bone_id, global_transforms)
}

/// A leaf joint extends its own bone: the world offset from the parent head, expressed in the bone frame.
fn leaf_bone_length_and_axis(
    skeleton: &Skeleton,
    bone_id: BoneId,
    global_transforms: &[Matrix4<f32>],
) -> (f32, Vector3<f32>) {
    let idx = bone_id as usize;
    let head = extract_world_position(&global_transforms[idx]);
    let parent_head = skeleton.bones[idx]
        .parent_id
        .and_then(|parent| global_transforms.get(parent as usize))
        .map_or(head, extract_world_position);
    let offset_from_parent = head - parent_head;
    let length = offset_from_parent.magnitude();
    if length <= 1e-6 {
        return (0.1, Vector3::new(0.0, 1.0, 0.0));
    }
    let (_, bone_rot, _) = decompose_transform(&global_transforms[idx]);
    let axis_in_bone_frame = rotate_vec3_by_quat(conjugate(bone_rot), offset_from_parent / length);
    (length, axis_in_bone_frame)
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

/// Collider radii are bone-local like their offsets (VRM 1.0), so they take the bone's scale.
fn bone_length_scale(bone_transform: &Matrix4<f32>) -> f32 {
    mean_axis_scale(&Matrix3::from_cols(
        bone_transform.x.truncate(),
        bone_transform.y.truncate(),
        bone_transform.z.truncate(),
    ))
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
            let radius = collider_shape_radius(&def.shape) * bone_length_scale(bone_transform);

            match &def.shape {
                ColliderShape::Sphere { .. } => {
                    world_colliders.push(WorldCollider {
                        center,
                        radius,
                        tail: None,
                    });
                }
                ColliderShape::Capsule { tail, .. } => {
                    let world_tail = transform_point(bone_transform, *tail);
                    world_colliders.push(WorldCollider {
                        center,
                        radius,
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

/// The chains whose joints skin the mesh of `entity`; empty for an entity without a skinned mesh.
pub fn chains_skinned_to_entity(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    setup: &SpringBoneSetup,
    entity: Entity,
) -> Vec<SpringChainId> {
    let Some(mesh_ref) = world.get_component::<MeshRef>(entity) else {
        return Vec::new();
    };
    let Some(mesh_index) = assets
        .get_mesh(mesh_ref.mesh_asset_id)
        .map(|mesh| mesh.graphics_mesh_index)
    else {
        return Vec::new();
    };
    match graphics
        .meshes
        .get(mesh_index)
        .and_then(|mesh| mesh.skin_data.as_ref())
    {
        Some(skin) => chains_skinned_to(setup, skin),
        None => Vec::new(),
    }
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

#[cfg(test)]
mod tests {
    use cgmath::{Deg, Rotation3};

    use super::*;

    fn leaf_tail_for_local_rotation(leaf_rotation: Quaternion<f32>) -> Vector3<f32> {
        let parent_rotation = Quaternion::from_angle_z(Deg(30.0));
        let parent_local = compose_transform(
            Vector3::new(0.0, 1.0, 0.0),
            parent_rotation,
            Vector3::new(1.0, 1.0, 1.0),
        );
        let leaf_local = compose_transform(
            Vector3::new(0.1, -0.2, 0.05),
            leaf_rotation,
            Vector3::new(1.0, 1.0, 1.0),
        );
        let globals = [parent_local, parent_local * leaf_local];
        let mut skeleton = Skeleton::new("leaf");
        let parent = skeleton.add_bone("parent", None);
        let leaf = skeleton.add_bone("leaf", Some(parent));

        let (length, axis) = leaf_bone_length_and_axis(&skeleton, leaf, &globals);
        let (_, leaf_local_rotation, _) = decompose_transform(&leaf_local);
        compute_tail_position(
            extract_world_position(&globals[1]),
            parent_rotation,
            leaf_local_rotation,
            axis,
            length,
        )
    }

    #[test]
    fn leaf_tail_does_not_depend_on_the_bone_local_frame() {
        let identity_frame = leaf_tail_for_local_rotation(Quaternion::new(1.0, 0.0, 0.0, 0.0));
        let rotated_frame = leaf_tail_for_local_rotation(
            Quaternion::from_angle_x(Deg(-98.0)) * Quaternion::from_angle_y(Deg(11.0)),
        );

        assert!(
            (identity_frame - rotated_frame).magnitude() < 1e-5,
            "{identity_frame:?} vs {rotated_frame:?}"
        );
    }

    #[test]
    fn leaf_extends_its_own_bone_direction() {
        let tail = leaf_tail_for_local_rotation(Quaternion::new(1.0, 0.0, 0.0, 0.0));
        let parent_rotation = Quaternion::from_angle_z(Deg(30.0));
        let offset = rotate_vec3_by_quat(parent_rotation, Vector3::new(0.1, -0.2, 0.05));
        let head = Vector3::new(0.0, 1.0, 0.0) + offset;

        assert!((tail - (head + offset)).magnitude() < 1e-5, "{tail:?}");
    }
}
