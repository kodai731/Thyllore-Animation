use cgmath::{InnerSpace, Matrix4, Quaternion, SquareMatrix, Vector3};

use super::{
    apply_length_constraint, compute_joint_rotation, compute_tail_position, conjugate,
    extract_world_position, integrate_joint, merge_collider_shrink, project_chain_clearance,
    recompute_global_transform, resolve_all_segment_collisions, resolve_all_tail_collisions,
    rotate_vector, segment_axis_distance, shrunk_collider_radius, tail_axis_distance,
    world_collider, ColliderShrink, JointForce, JointForces, JointRestFrame, JointRestGeometry,
    RestTolerance, SpringBoneSetup, SpringChain, SpringChainId, SpringCollisionMode,
    SpringJointParam, SpringSpace, WorldCollider, DEFAULT_SIMULATION_STEP,
};
use crate::{compose_transform, decompose_transform, BoneId, Skeleton, SkeletonPose};
use thyllore_math_core::{world_y_axis, world_y_down, EARTH_GRAVITY_M_PER_S2};

const REST_VELOCITY_WORLD_UNITS: f32 = 1e-4;
/// Push-out and length constraint passes per step; the length re-projection can re-enter a collider.
const COLLISION_PASSES: usize = 4;

#[derive(Clone, Debug)]
pub struct SpringJointState {
    pub prev_tail: Vector3<f32>,
    pub current_tail: Vector3<f32>,
    pub bone_length: f32,
    pub bone_axis: Vector3<f32>,
    pub initial_local_rotation: Quaternion<f32>,
    /// The hit radius the joint collides with, in skeleton units; a VRM joint's carries its rest clearance.
    pub hit_radius: f32,
    /// Distance from the rest tail to each collider axis of the chain, in collider order; empty for a VRM joint.
    pub rest_axis_distances: Vec<f32>,
    /// How much of the rest penetration the joint keeps: 1 at the strand root, 0 at its tip.
    pub rest_tolerance_weight: f32,
    /// The joint's gravity direction in the parent's frame at initialization.
    pub gravity_in_parent_rest: Vector3<f32>,
}

impl Default for SpringJointState {
    fn default() -> Self {
        Self {
            prev_tail: Vector3::new(0.0, 0.0, 0.0),
            current_tail: Vector3::new(0.0, 0.0, 0.0),
            bone_length: 0.0,
            bone_axis: world_y_axis(),
            initial_local_rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            hit_radius: 0.0,
            rest_axis_distances: Vec::new(),
            rest_tolerance_weight: 0.0,
            gravity_in_parent_rest: world_y_down(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpringChainState {
    pub chain_id: SpringChainId,
    pub joint_states: Vec<SpringJointState>,
    /// Sum of the chain's bone lengths in skeleton units.
    pub strand_length: f32,
}

/// The Verlet state of every enabled chain, one entry per enabled chain in setup order.
#[derive(Clone, Debug)]
pub struct SpringSimulation {
    pub chain_states: Vec<SpringChainState>,
    /// Colliders shrunk for VRM joints that rest inside them, in skeleton units.
    pub collider_shrinks: Vec<ColliderShrink>,
    pub max_delta_time: f32,
    pub frame_count: u32,
}

impl Default for SpringSimulation {
    fn default() -> Self {
        Self {
            chain_states: Vec::new(),
            collider_shrinks: Vec::new(),
            max_delta_time: DEFAULT_SIMULATION_STEP,
            frame_count: 0,
        }
    }
}

/// Rest state of every enabled chain; `space` scales the metre-unit radii into skeleton units.
pub fn spring_bone_initialize(
    setup: &SpringBoneSetup,
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    space: SpringSpace,
) -> SpringSimulation {
    let mut collider_shrinks = Vec::new();
    let chain_states = setup
        .chains
        .iter()
        .filter(|chain| chain.enabled)
        .map(|chain| {
            let (chain_state, shrinks) =
                initialize_chain(setup, chain, skeleton, global_transforms, space);
            for shrink in shrinks {
                merge_collider_shrink(&mut collider_shrinks, shrink);
            }
            chain_state
        })
        .collect();

    SpringSimulation {
        chain_states,
        collider_shrinks,
        ..Default::default()
    }
}

fn initialize_chain(
    setup: &SpringBoneSetup,
    chain: &SpringChain,
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    space: SpringSpace,
) -> (SpringChainState, Vec<ColliderShrink>) {
    let chain_colliders =
        collect_world_colliders(setup, &chain.collider_group_ids, global_transforms, &[]);
    let world_colliders: Vec<WorldCollider> = chain_colliders
        .iter()
        .map(|(_, collider)| *collider)
        .collect();
    let mut joint_states: Vec<SpringJointState> = chain
        .joints
        .iter()
        .enumerate()
        .filter(|(_, joint)| {
            let idx = joint.bone_id as usize;
            idx < skeleton.bones.len() && idx < global_transforms.len()
        })
        .map(|(joint_index, joint)| {
            initialize_joint(
                setup,
                chain,
                joint_index,
                joint,
                skeleton,
                global_transforms,
                &world_colliders,
                space,
            )
        })
        .collect();

    let strand_length = joint_states.iter().map(|joint| joint.bone_length).sum();
    weight_rest_tolerance_along_strand(&mut joint_states, strand_length);
    let shrinks = project_vrm_joints(chain, &mut joint_states, &chain_colliders);
    let chain_state = SpringChainState {
        chain_id: chain.id,
        joint_states,
        strand_length,
    };
    (chain_state, shrinks)
}

/// VRM joints have no rest tolerance at runtime (UniVRM), so theirs is projected onto the hit radius
/// and, for a joint resting inside a collider, onto that collider's radius.
fn project_vrm_joints(
    chain: &SpringChain,
    joint_states: &mut [SpringJointState],
    chain_colliders: &[(u32, WorldCollider)],
) -> Vec<ColliderShrink> {
    let vrm_joints: Vec<(usize, JointRestGeometry)> = chain
        .joints
        .iter()
        .zip(joint_states.iter())
        .enumerate()
        .filter(|(_, (joint, _))| matches!(joint.force, JointForce::Vrm(_)))
        .map(|(index, (joint, state))| {
            let geometry = JointRestGeometry {
                bone_id: joint.bone_id,
                hit_radius: state.hit_radius,
                rest_axis_distances: state.rest_axis_distances.clone(),
                tolerance_weight: state.rest_tolerance_weight,
            };
            (index, geometry)
        })
        .collect();
    if vrm_joints.is_empty() {
        return Vec::new();
    }

    let collider_radii: Vec<(u32, f32)> = chain_colliders
        .iter()
        .map(|(id, collider)| (*id, collider.radius))
        .collect();
    let geometries: Vec<JointRestGeometry> = vrm_joints.iter().map(|(_, g)| g.clone()).collect();
    let clearance = project_chain_clearance(chain.id, &geometries, &collider_radii);

    for ((index, _), hit_radius) in vrm_joints.iter().zip(clearance.hit_radii) {
        joint_states[*index].hit_radius = hit_radius;
        joint_states[*index].rest_axis_distances.clear();
    }
    clearance.shrinks
}

/// Fades the tolerated rest penetration from the strand root to its tip, by arc length at each tail.
fn weight_rest_tolerance_along_strand(joint_states: &mut [SpringJointState], strand_length: f32) {
    if strand_length <= 0.0 {
        return;
    }
    let mut arc_length = 0.0;
    for joint_state in joint_states {
        arc_length += joint_state.bone_length;
        joint_state.rest_tolerance_weight = 1.0 - arc_length / strand_length;
    }
}

fn initialize_joint(
    setup: &SpringBoneSetup,
    chain: &SpringChain,
    joint_index: usize,
    joint_param: &SpringJointParam,
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
    world_colliders: &[WorldCollider],
    space: SpringSpace,
) -> SpringJointState {
    let bone_id = joint_param.bone_id;
    let idx = bone_id as usize;
    let (_, initial_local_rotation, _) = decompose_transform(&skeleton.bones[idx].local_transform);

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

    let rest_axis_distances =
        rest_axis_distances(head_pos, tail, world_colliders, setup.collision_mode);
    let parent_world_rotation = extract_parent_world_rotation(skeleton, idx, global_transforms);
    let gravity_in_parent_rest =
        rotate_vector(conjugate(parent_world_rotation), joint_param.gravity_dir);

    SpringJointState {
        prev_tail: tail,
        current_tail: tail,
        bone_length,
        bone_axis,
        initial_local_rotation,
        hit_radius: space.skeleton_length(joint_param.hit_radius),
        rest_axis_distances,
        rest_tolerance_weight: 0.0,
        gravity_in_parent_rest,
    }
}

pub fn spring_bone_update(
    setup: &SpringBoneSetup,
    simulation: &mut SpringSimulation,
    skeleton: &Skeleton,
    global_transforms: &mut [Matrix4<f32>],
    pose: &SkeletonPose,
    dt: f32,
    space: SpringSpace,
) {
    let dt = dt.min(simulation.max_delta_time);
    if dt <= 0.0 {
        return;
    }

    let enabled_chains = setup.chains.iter().filter(|c| c.enabled);
    for (chain, chain_state) in enabled_chains.zip(&mut simulation.chain_states) {
        let step = ChainStep {
            skeleton,
            pose,
            world_colliders: collect_world_colliders(
                setup,
                &chain.collider_group_ids,
                global_transforms,
                &simulation.collider_shrinks,
            )
            .into_iter()
            .map(|(_, collider)| collider)
            .collect(),
            strand_length: chain_state.strand_length,
            dt,
            space,
            collision_mode: setup.collision_mode,
        };

        for (joint_param, joint_state) in chain.joints.iter().zip(&mut chain_state.joint_states) {
            update_single_joint(joint_param, joint_state, global_transforms, &step);
        }
    }

    simulation.frame_count += 1;
}

/// What every joint of one chain shares during a step.
struct ChainStep<'a> {
    skeleton: &'a Skeleton,
    pose: &'a SkeletonPose,
    world_colliders: Vec<WorldCollider>,
    strand_length: f32,
    dt: f32,
    space: SpringSpace,
    collision_mode: SpringCollisionMode,
}

fn update_single_joint(
    joint_param: &SpringJointParam,
    joint_state: &mut SpringJointState,
    global_transforms: &mut [Matrix4<f32>],
    step: &ChainStep,
) {
    let ChainStep {
        skeleton,
        pose,
        strand_length,
        dt,
        space,
        collision_mode,
        ..
    } = *step;
    let world_colliders = step.world_colliders.as_slice();
    let bone_id = joint_param.bone_id;
    let idx = bone_id as usize;
    if idx >= skeleton.bones.len() || idx >= global_transforms.len() {
        return;
    }

    let head_pos = extract_world_position(&global_transforms[idx]);
    let parent_world_rot = extract_parent_world_rotation(skeleton, idx, global_transforms);
    let hit_radius = joint_state.hit_radius;
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
    let collision = JointCollision {
        head: head_pos,
        hit_radius,
        bone_length: joint_state.bone_length,
        world_colliders,
        rest_tolerance: RestTolerance {
            axis_distances: &joint_state.rest_axis_distances,
            weight: joint_state.rest_tolerance_weight,
        },
        collision_mode,
    };
    let resolved_tail = resolve_collisions_at_bone_length(constrained_tail, &collision);

    let collision_displacement = resolved_tail - constrained_tail;
    let velocity = constrained_tail - joint_state.current_tail;
    let rest_velocity = space.skeleton_length(REST_VELOCITY_WORLD_UNITS);
    if velocity.magnitude2() < rest_velocity * rest_velocity {
        joint_state.prev_tail = resolved_tail;
    } else {
        joint_state.prev_tail = joint_state.current_tail + collision_displacement;
    }
    joint_state.current_tail = resolved_tail;

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

/// What one joint's tail is resolved against in a step.
struct JointCollision<'a> {
    head: Vector3<f32>,
    hit_radius: f32,
    bone_length: f32,
    world_colliders: &'a [WorldCollider],
    rest_tolerance: RestTolerance<'a>,
    collision_mode: SpringCollisionMode,
}

/// Alternates collider push-out and the length constraint until the tail sits outside at bone length.
fn resolve_collisions_at_bone_length(
    tail: Vector3<f32>,
    collision: &JointCollision,
) -> Vector3<f32> {
    let mut resolved_tail = tail;
    for _ in 0..COLLISION_PASSES {
        let pushed_tail = match collision.collision_mode {
            SpringCollisionMode::VrmCompatible => resolve_all_tail_collisions(
                resolved_tail,
                collision.hit_radius,
                collision.world_colliders,
                collision.rest_tolerance,
            ),
            SpringCollisionMode::SegmentExtended => resolve_all_segment_collisions(
                collision.head,
                resolved_tail,
                collision.hit_radius,
                collision.world_colliders,
                collision.rest_tolerance,
            ),
        };
        if pushed_tail == resolved_tail {
            return resolved_tail;
        }
        resolved_tail = apply_length_constraint(collision.head, pushed_tail, collision.bone_length);
    }
    resolved_tail
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

        let parent_global = parent_global_transform(skeleton, idx, global_transforms);
        let inv_parent = parent_global.invert().unwrap_or_else(Matrix4::identity);
        let (t, r, s) = decompose_transform(&(inv_parent * global_transforms[idx]));
        pose.bone_poses[idx].translation = t;
        pose.bone_poses[idx].rotation = r;
        pose.bone_poses[idx].scale = s;
    }
}

/// Moves every tail to where the bones point in `global_transforms` without touching the rest data.
pub fn spring_bone_reseat_tails(
    setup: &SpringBoneSetup,
    simulation: &mut SpringSimulation,
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
) {
    let enabled_chains = setup.chains.iter().filter(|chain| chain.enabled);
    for (chain, chain_state) in enabled_chains.zip(&mut simulation.chain_states) {
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

fn joint_forces(
    joint_param: &SpringJointParam,
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
fn joint_tail_bone(chain: &SpringChain, joint_index: usize, skeleton: &Skeleton) -> Option<BoneId> {
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

fn compute_initial_tail_position(
    skeleton: &Skeleton,
    bone_id: BoneId,
    tail_bone: Option<BoneId>,
    global_transforms: &[Matrix4<f32>],
    initial_local_rotation: Quaternion<f32>,
    bone_axis: Vector3<f32>,
    bone_length: f32,
) -> Vector3<f32> {
    if let Some(tail_transform) = tail_bone.and_then(|id| global_transforms.get(id as usize)) {
        return extract_world_position(tail_transform);
    }

    let idx = bone_id as usize;
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
    let parent_transform = match skeleton.bones[bone_idx].parent_id {
        Some(parent_id) => match global_transforms.get(parent_id as usize) {
            Some(transform) => *transform,
            None => return Quaternion::new(1.0, 0.0, 0.0, 0.0),
        },
        None => skeleton.root_transform,
    };
    let (_, rotation, _) = decompose_transform(&parent_transform);
    rotation
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
        let diff = extract_world_position(tail_transform) - head_pos;
        let length = diff.magnitude();
        if length > 1e-6 {
            let (_, bone_rot, _) = decompose_transform(&global_transforms[idx]);
            return (length, rotate_vector(conjugate(bone_rot), diff / length));
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
        return (0.1, world_y_axis());
    }
    let (_, bone_rot, _) = decompose_transform(&global_transforms[idx]);
    (
        length,
        rotate_vector(conjugate(bone_rot), offset_from_parent / length),
    )
}

fn parent_global_transform(
    skeleton: &Skeleton,
    bone_idx: usize,
    global_transforms: &[Matrix4<f32>],
) -> Matrix4<f32> {
    match skeleton.bones[bone_idx].parent_id {
        Some(parent_id) => global_transforms
            .get(parent_id as usize)
            .copied()
            .unwrap_or_else(Matrix4::identity),
        None => skeleton.root_transform,
    }
}

fn update_global_transforms_for_bone(
    skeleton: &Skeleton,
    bone_id: BoneId,
    new_local_rotation: Quaternion<f32>,
    global_transforms: &mut [Matrix4<f32>],
    pose: &SkeletonPose,
) {
    let idx = bone_id as usize;
    if idx >= skeleton.bones.len() {
        return;
    }

    let parent_global = parent_global_transform(skeleton, idx, global_transforms);
    global_transforms[idx] = recompute_global_transform(
        &parent_global,
        new_local_rotation,
        &skeleton.bones[idx].local_transform,
    );

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

    let parent_global = parent_global_transform(skeleton, idx, global_transforms);
    let animated_local = match pose.bone_poses.get(idx) {
        Some(bone_pose) => {
            compose_transform(bone_pose.translation, bone_pose.rotation, bone_pose.scale)
        }
        None => skeleton.bones[idx].local_transform,
    };
    global_transforms[idx] = parent_global * animated_local;

    for &child_id in &skeleton.bones[idx].children {
        recompute_children_globals(skeleton, child_id, global_transforms, pose);
    }
}

fn collect_world_colliders(
    setup: &SpringBoneSetup,
    collider_group_ids: &[u32],
    global_transforms: &[Matrix4<f32>],
    shrinks: &[ColliderShrink],
) -> Vec<(u32, WorldCollider)> {
    let collider_ids = collider_group_ids.iter().flat_map(|group_id| {
        setup
            .collider_groups
            .iter()
            .filter(move |group| group.id == *group_id)
            .flat_map(|group| group.collider_ids.iter().copied())
    });

    collider_ids
        .filter_map(|collider_id| setup.colliders.iter().find(|c| c.id == collider_id))
        .filter_map(|def| {
            let transform = global_transforms.get(def.bone_id as usize)?;
            let mut collider = world_collider(def, transform);
            collider.radius = shrunk_collider_radius(shrinks, def.id, collider.radius);
            Some((def.id, collider))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use cgmath::{Deg, Rotation3};

    use super::*;
    use crate::spring_bone::{
        ColliderGroupOrigin, ColliderShape, RestRelativeJointForce, SpringColliderDef,
        SpringColliderGroup, VrmJointForce,
    };
    use crate::{compute_pose_global_transforms, create_pose_from_rest};

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
        let offset = rotate_vector(parent_rotation, Vector3::new(0.1, -0.2, 0.05));
        let head = Vector3::new(0.0, 1.0, 0.0) + offset;

        assert!((tail - (head + offset)).magnitude() < 1e-5, "{tail:?}");
    }

    fn straight_strand(bone_count: usize) -> Skeleton {
        let mut skeleton = Skeleton::new("strand");
        let mut parent = None;
        for index in 0..bone_count {
            let bone_id = skeleton.add_bone(&format!("bone_{index}"), parent);
            skeleton.bones[bone_id as usize].local_transform =
                Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0));
            parent = Some(bone_id);
        }
        skeleton
    }

    fn chain_of(bone_ids: &[BoneId]) -> SpringChain {
        SpringChain {
            joints: bone_ids
                .iter()
                .map(|&bone_id| SpringJointParam {
                    bone_id,
                    force: JointForce::RestRelative(RestRelativeJointForce {
                        bending_relaxation: 0.0,
                        gravity_scale: 1.0,
                    }),
                    drag_force: 0.0,
                    gravity_dir: Vector3::new(1.0, 0.0, 0.0),
                    hit_radius: 0.1,
                })
                .collect(),
            collider_group_ids: vec![0],
            ..Default::default()
        }
    }

    #[test]
    fn rest_tolerance_fades_from_the_strand_root_to_its_tip() {
        let skeleton = straight_strand(4);
        let setup = SpringBoneSetup {
            chains: vec![chain_of(&[1, 2])],
            ..Default::default()
        };
        let globals = compute_pose_global_transforms(&skeleton, &create_pose_from_rest(&skeleton));

        let simulation = spring_bone_initialize(&setup, &skeleton, &globals, SpringSpace::IDENTITY);

        let weights: Vec<f32> = simulation.chain_states[0]
            .joint_states
            .iter()
            .map(|joint| joint.rest_tolerance_weight)
            .collect();
        assert!((weights[0] - 0.5).abs() < 1e-6, "{weights:?}");
        assert!(weights[1].abs() < 1e-6, "{weights:?}");
    }

    fn vrm_chain_of(bone_ids: &[BoneId]) -> SpringChain {
        let mut chain = chain_of(bone_ids);
        for joint in &mut chain.joints {
            joint.force = JointForce::Vrm(VrmJointForce {
                stiffness: 0.0,
                gravity_power: 0.0,
            });
        }
        chain
    }

    fn sphere_group_at(center: Vector3<f32>, radius: f32) -> SpringBoneSetup {
        SpringBoneSetup {
            colliders: vec![SpringColliderDef {
                id: 0,
                bone_id: 0,
                offset: center,
                shape: ColliderShape::Sphere { radius },
            }],
            collider_groups: vec![SpringColliderGroup {
                id: 0,
                name: "body".to_string(),
                collider_ids: vec![0],
                origin: ColliderGroupOrigin::Authored,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn a_vrm_root_joint_carries_its_rest_penetration_in_its_hit_radius() {
        let skeleton = straight_strand(4);
        let mut setup = sphere_group_at(Vector3::new(0.25, 2.0, 0.0), 0.2);
        setup.chains = vec![vrm_chain_of(&[1, 2])];
        let globals = compute_pose_global_transforms(&skeleton, &create_pose_from_rest(&skeleton));

        let simulation = spring_bone_initialize(&setup, &skeleton, &globals, SpringSpace::IDENTITY);

        let root = &simulation.chain_states[0].joint_states[0];
        assert!(root.rest_axis_distances.is_empty());
        assert!(
            (root.hit_radius - 0.075).abs() < 1e-5,
            "{}",
            root.hit_radius
        );
        assert!(simulation.collider_shrinks.is_empty());
    }

    #[test]
    fn a_vrm_joint_resting_inside_a_collider_shrinks_it() {
        let skeleton = straight_strand(4);
        let mut setup = sphere_group_at(Vector3::new(0.02, 2.0, 0.0), 0.2);
        setup.chains = vec![vrm_chain_of(&[1, 2])];
        let globals = compute_pose_global_transforms(&skeleton, &create_pose_from_rest(&skeleton));

        let simulation = spring_bone_initialize(&setup, &skeleton, &globals, SpringSpace::IDENTITY);

        let root = &simulation.chain_states[0].joint_states[0];
        assert_eq!(root.hit_radius, 0.0);
        let shrink = simulation.collider_shrinks[0];
        assert_eq!((shrink.collider_id, shrink.bone_id), (0, 1));
        assert!((shrink.to - 0.16).abs() < 1e-5, "{shrink:?}");
    }

    #[test]
    fn a_vrm_chain_at_rest_stays_where_a_rest_relative_chain_does() {
        let skeleton = straight_strand(4);
        let mut setup = sphere_group_at(Vector3::new(0.1, 2.0, 0.0), 0.2);
        let mut rest_relative = chain_of(&[1, 2]);
        for joint in &mut rest_relative.joints {
            joint.force = JointForce::RestRelative(RestRelativeJointForce {
                bending_relaxation: 0.0,
                gravity_scale: 0.0,
            });
        }
        setup.chains = vec![rest_relative, vrm_chain_of(&[1, 2])];
        let pose = create_pose_from_rest(&skeleton);
        let rest_globals = compute_pose_global_transforms(&skeleton, &pose);
        let mut simulation =
            spring_bone_initialize(&setup, &skeleton, &rest_globals, SpringSpace::IDENTITY);

        for _ in 0..3 {
            let mut globals = rest_globals.clone();
            spring_bone_update(
                &setup,
                &mut simulation,
                &skeleton,
                &mut globals,
                &pose,
                DEFAULT_SIMULATION_STEP,
                SpringSpace::IDENTITY,
            );
        }

        let rest_relative_tails: Vec<Vector3<f32>> = simulation.chain_states[0]
            .joint_states
            .iter()
            .map(|joint| joint.current_tail)
            .collect();
        let vrm_tails: Vec<Vector3<f32>> = simulation.chain_states[1]
            .joint_states
            .iter()
            .map(|joint| joint.current_tail)
            .collect();
        for (rest_relative, vrm) in rest_relative_tails.iter().zip(&vrm_tails) {
            assert!(
                (rest_relative - vrm).magnitude() < 1e-4,
                "{rest_relative_tails:?} vs {vrm_tails:?}"
            );
        }
    }

    #[test]
    fn a_collider_push_out_adds_no_velocity() {
        let skeleton = straight_strand(3);
        let setup = SpringBoneSetup {
            chains: vec![chain_of(&[1])],
            colliders: vec![SpringColliderDef {
                id: 0,
                bone_id: 0,
                offset: Vector3::new(0.0, 2.0, 0.0),
                shape: ColliderShape::Sphere { radius: 0.5 },
            }],
            collider_groups: vec![SpringColliderGroup {
                id: 0,
                name: "head".to_string(),
                collider_ids: vec![0],
                origin: ColliderGroupOrigin::Authored,
            }],
            ..Default::default()
        };
        let pose = create_pose_from_rest(&skeleton);
        let mut globals = compute_pose_global_transforms(&skeleton, &pose);
        let mut simulation =
            spring_bone_initialize(&setup, &skeleton, &globals, SpringSpace::IDENTITY);

        let dt = DEFAULT_SIMULATION_STEP;
        spring_bone_update(
            &setup,
            &mut simulation,
            &skeleton,
            &mut globals,
            &pose,
            dt,
            SpringSpace::IDENTITY,
        );

        let joint = &simulation.chain_states[0].joint_states[0];
        let collider_center = Vector3::new(0.0, 2.0, 0.0);
        let clearance = (joint.current_tail - collider_center).magnitude();
        assert!(
            clearance >= 0.6 - 1e-4,
            "tip tail still inside: {clearance}"
        );
        let gravity_step = EARTH_GRAVITY_M_PER_S2 * dt * dt;
        let velocity = (joint.current_tail - joint.prev_tail).magnitude();
        assert!(
            velocity <= gravity_step + 1e-4,
            "push-out leaked into velocity: {velocity}"
        );
    }

    #[test]
    fn push_out_survives_the_length_re_projection() {
        let colliders = [WorldCollider {
            center: Vector3::new(0.0, 0.0, 0.0),
            radius: 1.0,
            tail: None,
        }];
        let head = Vector3::new(1.1, 0.0, 0.0);
        let tail = Vector3::new(0.2, -0.5, 0.0);
        let collision = JointCollision {
            head,
            hit_radius: 0.0,
            bone_length: (tail - head).magnitude(),
            world_colliders: &colliders,
            rest_tolerance: RestTolerance::NONE,
            collision_mode: SpringCollisionMode::VrmCompatible,
        };

        let single_pass = apply_length_constraint(
            head,
            resolve_all_tail_collisions(tail, 0.0, &colliders, RestTolerance::NONE),
            collision.bone_length,
        );
        let resolved = resolve_collisions_at_bone_length(tail, &collision);

        assert!(single_pass.magnitude() < 1.0 - 1e-3, "{single_pass:?}");
        assert!(resolved.magnitude() >= 1.0 - 1e-3, "{resolved:?}");
        assert!(((resolved - head).magnitude() - collision.bone_length).abs() < 1e-5);
    }
}
