use cgmath::{InnerSpace, Matrix4, Quaternion, SquareMatrix, Vector3};

use super::{
    apply_length_constraint, compute_joint_rotation, compute_tail_position, conjugate,
    extract_world_position, integrate_joint, recompute_global_transform,
    resolve_all_segment_collisions, resolve_all_tail_collisions, rotate_vector,
    segment_axis_distance, tail_axis_distance, world_collider, JointForce, JointForces,
    JointRestFrame, SpringBoneSetup, SpringChain, SpringChainId, SpringCollisionMode,
    SpringJointParam, SpringSpace, WorldCollider, DEFAULT_SIMULATION_STEP, EARTH_GRAVITY_M_PER_S2,
};
use crate::{compose_transform, decompose_transform, BoneId, Skeleton, SkeletonPose};

const REST_VELOCITY_WORLD_UNITS: f32 = 1e-4;

#[derive(Clone, Debug)]
pub struct SpringJointState {
    pub prev_tail: Vector3<f32>,
    pub current_tail: Vector3<f32>,
    pub bone_length: f32,
    pub bone_axis: Vector3<f32>,
    pub initial_local_rotation: Quaternion<f32>,
    /// Distance from the rest tail to each collider axis of the chain, in collider order.
    pub rest_axis_distances: Vec<f32>,
    /// The joint's gravity direction in the parent's frame at initialization.
    pub gravity_in_parent_rest: Vector3<f32>,
}

impl Default for SpringJointState {
    fn default() -> Self {
        Self {
            prev_tail: Vector3::new(0.0, 0.0, 0.0),
            current_tail: Vector3::new(0.0, 0.0, 0.0),
            bone_length: 0.0,
            bone_axis: Vector3::new(0.0, 1.0, 0.0),
            initial_local_rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            rest_axis_distances: Vec::new(),
            gravity_in_parent_rest: Vector3::new(0.0, -1.0, 0.0),
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
    pub max_delta_time: f32,
    pub frame_count: u32,
}

impl Default for SpringSimulation {
    fn default() -> Self {
        Self {
            chain_states: Vec::new(),
            max_delta_time: DEFAULT_SIMULATION_STEP,
            frame_count: 0,
        }
    }
}

pub fn spring_bone_initialize(
    setup: &SpringBoneSetup,
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
) -> SpringSimulation {
    let chain_states = setup
        .chains
        .iter()
        .filter(|chain| chain.enabled)
        .map(|chain| initialize_chain(setup, chain, skeleton, global_transforms))
        .collect();

    SpringSimulation {
        chain_states,
        ..Default::default()
    }
}

fn initialize_chain(
    setup: &SpringBoneSetup,
    chain: &SpringChain,
    skeleton: &Skeleton,
    global_transforms: &[Matrix4<f32>],
) -> SpringChainState {
    let world_colliders =
        collect_world_colliders(setup, &chain.collider_group_ids, global_transforms);
    let joint_states: Vec<SpringJointState> = chain
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
            )
        })
        .collect();

    let strand_length = joint_states.iter().map(|joint| joint.bone_length).sum();
    SpringChainState {
        chain_id: chain.id,
        joint_states,
        strand_length,
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

    let rest_axis_distances = match joint_param.force {
        JointForce::RestRelative(_) => {
            rest_axis_distances(head_pos, tail, world_colliders, setup.collision_mode)
        }
        JointForce::Vrm(_) => Vec::new(),
    };
    let parent_world_rotation = extract_parent_world_rotation(skeleton, idx, global_transforms);
    let gravity_in_parent_rest =
        rotate_vector(conjugate(parent_world_rotation), joint_param.gravity_dir);

    SpringJointState {
        prev_tail: tail,
        current_tail: tail,
        bone_length,
        bone_axis,
        initial_local_rotation,
        rest_axis_distances,
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
            ),
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
    let resolved_tail =
        apply_length_constraint(head_pos, collision_resolved, joint_state.bone_length);

    let velocity = resolved_tail - joint_state.current_tail;
    let rest_velocity = space.skeleton_length(REST_VELOCITY_WORLD_UNITS);
    if velocity.magnitude2() < rest_velocity * rest_velocity {
        joint_state.prev_tail = resolved_tail;
    } else {
        joint_state.prev_tail = joint_state.current_tail;
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
        return (0.1, Vector3::new(0.0, 1.0, 0.0));
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
) -> Vec<WorldCollider> {
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
            global_transforms
                .get(def.bone_id as usize)
                .map(|transform| world_collider(def, transform))
        })
        .collect()
}

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
        let offset = rotate_vector(parent_rotation, Vector3::new(0.1, -0.2, 0.05));
        let head = Vector3::new(0.0, 1.0, 0.0) + offset;

        assert!((tail - (head + offset)).magnitude() < 1e-5, "{tail:?}");
    }
}
