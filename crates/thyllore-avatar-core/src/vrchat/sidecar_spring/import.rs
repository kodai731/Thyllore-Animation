use cgmath::Vector3;
use thyllore_anim_core::spring_bone::{
    ChainOrigin, ColliderShape, SpringBoneSetup, SpringChain, SpringColliderDef,
    SpringColliderGroup, SpringColliderGroupId, SpringColliderId, SpringCollisionMode,
    SpringForceModel, SpringJointParam, SubmeshSelector,
};
use thyllore_anim_core::BoneId;

use super::format::{
    SidecarChainOrigin, SidecarCollider, SidecarColliderGroup, SidecarColliderShape,
    SidecarCollisionMode, SidecarForceModel, SidecarSpring, SidecarSpringChain, SidecarSpringJoint,
};
use crate::humanoid::components::skeleton_input::BoneInput;

/// The engine setup a sidecar describes; joints and colliders on unknown bones are dropped.
pub fn spring_setup_from_sidecar(spring: &SidecarSpring, bones: &[BoneInput]) -> SpringBoneSetup {
    let mut setup = SpringBoneSetup {
        collision_mode: from_sidecar_collision_mode(&spring.collision_mode),
        force_model: from_sidecar_force_model(&spring.force_model),
        ..Default::default()
    };

    let collider_ids_by_name = import_colliders(&mut setup, &spring.colliders, bones);
    let group_ids_by_name =
        import_collider_groups(&mut setup, &spring.collider_groups, &collider_ids_by_name);
    for chain in &spring.chains {
        import_chain(&mut setup, chain, bones, &group_ids_by_name);
    }
    setup
}

fn from_sidecar_collision_mode(mode: &SidecarCollisionMode) -> SpringCollisionMode {
    match mode {
        SidecarCollisionMode::VrmCompatible => SpringCollisionMode::VrmCompatible,
        SidecarCollisionMode::SegmentExtended => SpringCollisionMode::SegmentExtended,
    }
}

fn from_sidecar_force_model(model: &SidecarForceModel) -> SpringForceModel {
    match model {
        SidecarForceModel::Vrm => SpringForceModel::Vrm,
        SidecarForceModel::RestRelative => SpringForceModel::RestRelative,
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
) -> Vec<(String, SpringColliderId)> {
    let mut ids_by_name = Vec::new();
    for collider in colliders {
        let Some(bone_id) = bone_id_of(bones, &collider.bone) else {
            continue;
        };
        let id = setup.next_collider_id;
        setup.next_collider_id += 1;
        setup.colliders.push(SpringColliderDef {
            id,
            bone_id,
            offset: Vector3::from(collider.offset),
            shape: collider_shape(collider),
        });
        ids_by_name.push((collider.name.clone(), id));
    }
    ids_by_name
}

/// A capsule without a tail degrades to a sphere.
fn collider_shape(collider: &SidecarCollider) -> ColliderShape {
    match (&collider.shape, collider.tail) {
        (SidecarColliderShape::Capsule, Some(tail)) => ColliderShape::Capsule {
            radius: collider.radius,
            tail: Vector3::from(tail),
        },
        (SidecarColliderShape::Capsule, None) | (SidecarColliderShape::Sphere, _) => {
            ColliderShape::Sphere {
                radius: collider.radius,
            }
        }
    }
}

fn import_collider_groups(
    setup: &mut SpringBoneSetup,
    groups: &[SidecarColliderGroup],
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
                stiffness: joint.stiffness,
                drag_force: joint.drag_force,
                gravity_power: joint.gravity_power,
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
        enabled: true,
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
