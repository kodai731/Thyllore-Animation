use cgmath::Vector3;
use thyllore_anim_core::spring_bone::{
    shared_force_model, ChainOrigin, ColliderGroupOrigin, ColliderShape, JointForce,
    SpringBoneSetup, SpringChain, SpringColliderDef, SpringColliderGroup, SpringColliderGroupId,
    SpringColliderId, SpringCollisionMode, SpringJointParam, SubmeshSelector,
};
use thyllore_anim_core::BoneId;

use super::format::{
    SidecarChainOrigin, SidecarCollider, SidecarColliderGroup, SidecarColliderOrigin,
    SidecarColliderShape, SidecarCollisionMode, SidecarSpring, SidecarSpringChain,
    SidecarSpringJoint,
};
use super::frame::BoneRestFrame;
use crate::humanoid::components::skeleton_input::BoneInput;

/// The engine setup a sidecar describes; joints and colliders on unknown bones are dropped.
/// `frames` gives each bone's rest frame in `bones` order and takes colliders back to bone space.
pub fn spring_setup_from_sidecar(
    spring: &SidecarSpring,
    bones: &[BoneInput],
    frames: &[BoneRestFrame],
) -> SpringBoneSetup {
    let mut setup = SpringBoneSetup {
        collision_mode: from_sidecar_collision_mode(&spring.collision_mode),
        ..Default::default()
    };

    let collider_ids_by_name = import_colliders(&mut setup, &spring.colliders, bones, frames);
    let group_ids_by_name = import_collider_groups(
        &mut setup,
        &spring.collider_groups,
        &spring.colliders,
        &collider_ids_by_name,
    );
    for chain in &spring.chains {
        import_chain(&mut setup, chain, bones, &group_ids_by_name);
    }
    setup.new_joint_force_model =
        shared_force_model(setup.chains.iter().flat_map(|chain| &chain.joints)).unwrap_or_default();
    setup
}

fn from_sidecar_collision_mode(mode: &SidecarCollisionMode) -> SpringCollisionMode {
    match mode {
        SidecarCollisionMode::VrmCompatible => SpringCollisionMode::VrmCompatible,
        SidecarCollisionMode::SegmentExtended => SpringCollisionMode::SegmentExtended,
    }
}

fn bone_id_of(bones: &[BoneInput], name: &str) -> Option<BoneId> {
    bones
        .iter()
        .position(|bone| bone.name == name)
        .map(|index| index as BoneId)
}

fn id_by_name<Id: Copy>(ids_by_name: &[(String, Id)], name: &str) -> Option<Id> {
    ids_by_name
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, id)| *id)
}

fn import_colliders(
    setup: &mut SpringBoneSetup,
    colliders: &[SidecarCollider],
    bones: &[BoneInput],
    frames: &[BoneRestFrame],
) -> Vec<(String, SpringColliderId)> {
    let mut ids_by_name = Vec::new();
    for collider in colliders {
        let Some(bone_id) = bone_id_of(bones, &collider.bone) else {
            continue;
        };
        let Some(frame) = frames.get(bone_id as usize) else {
            continue;
        };
        let id = setup.next_collider_id;
        setup.next_collider_id += 1;
        setup.colliders.push(SpringColliderDef {
            id,
            bone_id,
            offset: frame.to_local(Vector3::from(collider.offset)),
            shape: collider_shape(collider, frame),
        });
        ids_by_name.push((collider.name.clone(), id));
    }
    ids_by_name
}

/// The sidecar marks the origin per collider, so a group is fitted when every collider it holds is.
fn collider_group_origin(
    group: &SidecarColliderGroup,
    colliders: &[SidecarCollider],
) -> ColliderGroupOrigin {
    let all_fitted = !group.colliders.is_empty()
        && group.colliders.iter().all(|name| {
            colliders
                .iter()
                .any(|c| c.name == *name && c.origin == SidecarColliderOrigin::Fitted)
        });
    if all_fitted {
        ColliderGroupOrigin::Fitted
    } else {
        ColliderGroupOrigin::Authored
    }
}

/// A capsule without a tail degrades to a sphere.
fn collider_shape(collider: &SidecarCollider, frame: &BoneRestFrame) -> ColliderShape {
    let radius = frame.length_to_local(collider.radius);
    match (&collider.shape, collider.tail) {
        (SidecarColliderShape::Capsule, Some(tail)) => ColliderShape::Capsule {
            radius,
            tail: frame.to_local(Vector3::from(tail)),
        },
        (SidecarColliderShape::Capsule, None) | (SidecarColliderShape::Sphere, _) => {
            ColliderShape::Sphere { radius }
        }
    }
}

fn import_collider_groups(
    setup: &mut SpringBoneSetup,
    groups: &[SidecarColliderGroup],
    colliders: &[SidecarCollider],
    collider_ids_by_name: &[(String, SpringColliderId)],
) -> Vec<(String, SpringColliderGroupId)> {
    let mut ids_by_name = Vec::new();
    for group in groups {
        let id = setup.next_group_id;
        setup.next_group_id += 1;
        setup.collider_groups.push(SpringColliderGroup {
            id,
            name: group.name.clone(),
            collider_ids: group
                .colliders
                .iter()
                .filter_map(|name| id_by_name(collider_ids_by_name, name))
                .collect(),
            origin: collider_group_origin(group, colliders),
        });
        ids_by_name.push((group.name.clone(), id));
    }
    ids_by_name
}

fn import_chain(
    setup: &mut SpringBoneSetup,
    chain: &SidecarSpringChain,
    bones: &[BoneInput],
    group_ids_by_name: &[(String, SpringColliderGroupId)],
) {
    let joints: Vec<SpringJointParam> = simulated_joints(chain)
        .iter()
        .filter_map(|joint| {
            Some(SpringJointParam {
                bone_id: bone_id_of(bones, &joint.bone)?,
                force: JointForce::from(joint.force),
                drag_force: joint.drag_force,
                gravity_dir: Vector3::from(joint.gravity_dir),
                hit_radius: joint.hit_radius,
            })
        })
        .collect();
    if joints.is_empty() {
        return;
    }

    let id = setup.next_chain_id;
    setup.next_chain_id += 1;
    setup.chains.push(SpringChain {
        id,
        name: chain.name.clone(),
        joints,
        collider_group_ids: chain
            .collider_groups
            .iter()
            .filter_map(|name| id_by_name(group_ids_by_name, name))
            .collect(),
        center_bone_id: chain
            .center
            .as_deref()
            .and_then(|name| bone_id_of(bones, name)),
        tail_bone_id: chain
            .tail
            .as_deref()
            .and_then(|name| bone_id_of(bones, name)),
        enabled: chain.enabled,
        origin: chain_origin_from_sidecar(&chain.origin),
    });
}

/// The joints the engine simulates: the appended VRM tail joint is not one of them.
fn simulated_joints(chain: &SidecarSpringChain) -> &[SidecarSpringJoint] {
    match &chain.tail {
        Some(tail) if chain.joints.last().is_some_and(|last| &last.bone == tail) => {
            &chain.joints[..chain.joints.len() - 1]
        }
        _ => &chain.joints[..],
    }
}

fn chain_origin_from_sidecar(origin: &Option<SidecarChainOrigin>) -> ChainOrigin {
    match origin {
        None => ChainOrigin::Authored,
        Some(origin) => ChainOrigin::Generated {
            target: SubmeshSelector::from_parts(&origin.mesh, origin.material.as_deref()),
            role: origin.role,
        },
    }
}
