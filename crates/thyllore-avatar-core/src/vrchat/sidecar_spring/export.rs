use thyllore_anim_core::spring_bone::{
    ChainOrigin, ColliderGroupOrigin, ColliderShape, SpringBoneSetup, SpringChain,
    SpringColliderDef, SpringColliderId, SpringCollisionMode, SpringJointParam,
    FITTED_BODY_GROUP_NAME,
};

use super::format::{
    SidecarChainOrigin, SidecarCollider, SidecarColliderGroup, SidecarColliderOrigin,
    SidecarColliderShape, SidecarCollisionMode, SidecarJointForce, SidecarSpring,
    SidecarSpringChain, SidecarSpringJoint,
};
use super::frame::BoneRestFrame;
use crate::humanoid::components::skeleton_input::BoneInput;

/// `frames` gives each bone's rest frame in `bones` order; colliders are written in model space through it.
pub fn build_sidecar_spring(
    setup: &SpringBoneSetup,
    bones: &[BoneInput],
    frames: &[BoneRestFrame],
) -> SidecarSpring {
    let named_colliders = build_sidecar_colliders(setup, bones, frames);
    let collider_groups = build_sidecar_collider_groups(setup, &named_colliders);
    let chains = setup
        .chains
        .iter()
        .map(|chain| build_sidecar_chain(setup, chain, bones))
        .collect();

    SidecarSpring {
        collision_mode: to_sidecar_collision_mode(setup.collision_mode),
        colliders: named_colliders.into_iter().map(|(_, c)| c).collect(),
        collider_groups,
        chains,
    }
}

fn to_sidecar_collision_mode(mode: SpringCollisionMode) -> SidecarCollisionMode {
    match mode {
        SpringCollisionMode::VrmCompatible => SidecarCollisionMode::VrmCompatible,
        SpringCollisionMode::SegmentExtended => SidecarCollisionMode::SegmentExtended,
    }
}

fn bone_name(bones: &[BoneInput], bone_id: u32) -> Option<&str> {
    bones.get(bone_id as usize).map(|bone| bone.name.as_str())
}

fn build_sidecar_colliders(
    setup: &SpringBoneSetup,
    bones: &[BoneInput],
    frames: &[BoneRestFrame],
) -> Vec<(SpringColliderId, SidecarCollider)> {
    let fitted_ids: Vec<SpringColliderId> = setup
        .collider_groups
        .iter()
        .filter(|group| group.origin == ColliderGroupOrigin::Fitted)
        .flat_map(|group| group.collider_ids.iter().copied())
        .collect();

    setup
        .colliders
        .iter()
        .filter_map(|collider| {
            let bone = bone_name(bones, collider.bone_id)?;
            let frame = frames.get(collider.bone_id as usize)?;
            let origin = if fitted_ids.contains(&collider.id) {
                SidecarColliderOrigin::Fitted
            } else {
                SidecarColliderOrigin::Authored
            };
            Some((
                collider.id,
                build_sidecar_collider(collider, bone, frame, origin),
            ))
        })
        .collect()
}

fn build_sidecar_collider(
    collider: &SpringColliderDef,
    bone: &str,
    frame: &BoneRestFrame,
    origin: SidecarColliderOrigin,
) -> SidecarCollider {
    let name = match origin {
        SidecarColliderOrigin::Fitted => format!("{FITTED_BODY_GROUP_NAME}/{bone}"),
        SidecarColliderOrigin::Authored => format!("{bone}#{}", collider.id),
    };
    let (shape, radius, tail) = match &collider.shape {
        ColliderShape::Sphere { radius } => (
            SidecarColliderShape::Sphere,
            frame.length_to_model(*radius),
            None,
        ),
        ColliderShape::Capsule { radius, tail } => (
            SidecarColliderShape::Capsule,
            frame.length_to_model(*radius),
            Some(frame.to_model(*tail).into()),
        ),
    };
    SidecarCollider {
        name,
        bone: bone.to_string(),
        offset: frame.to_model(collider.offset).into(),
        shape,
        radius,
        tail,
        origin,
    }
}

fn collider_name(
    colliders: &[(SpringColliderId, SidecarCollider)],
    id: SpringColliderId,
) -> Option<String> {
    colliders
        .iter()
        .find(|(candidate, _)| *candidate == id)
        .map(|(_, collider)| collider.name.clone())
}

fn build_sidecar_collider_groups(
    setup: &SpringBoneSetup,
    colliders: &[(SpringColliderId, SidecarCollider)],
) -> Vec<SidecarColliderGroup> {
    setup
        .collider_groups
        .iter()
        .map(|group| SidecarColliderGroup {
            name: group.name.clone(),
            colliders: group
                .collider_ids
                .iter()
                .filter_map(|&id| collider_name(colliders, id))
                .collect(),
        })
        .collect()
}

fn build_sidecar_chain(
    setup: &SpringBoneSetup,
    chain: &SpringChain,
    bones: &[BoneInput],
) -> SidecarSpringChain {
    let mut joints: Vec<SidecarSpringJoint> = chain
        .joints
        .iter()
        .filter_map(|joint| {
            let bone = bone_name(bones, joint.bone_id)?;
            Some(build_sidecar_joint(joint, bone))
        })
        .collect();
    let tail = joints
        .last()
        .and_then(|last| build_tail_joint(chain, last, bones));
    let tail_bone = tail.as_ref().map(|joint| joint.bone.clone());
    joints.extend(tail);

    SidecarSpringChain {
        name: chain.name.clone(),
        center: chain
            .center_bone_id
            .and_then(|bone_id| bone_name(bones, bone_id))
            .map(str::to_string),
        collider_groups: collider_group_names(setup, chain),
        joints,
        tail: tail_bone,
        enabled: chain.enabled,
        origin: sidecar_chain_origin(&chain.origin),
    }
}

fn build_sidecar_joint(joint: &SpringJointParam, bone: &str) -> SidecarSpringJoint {
    SidecarSpringJoint {
        bone: bone.to_string(),
        force: SidecarJointForce::from(joint.force),
        drag_force: joint.drag_force,
        gravity_dir: [
            joint.gravity_dir.x,
            joint.gravity_dir.y,
            joint.gravity_dir.z,
        ],
        hit_radius: joint.hit_radius,
    }
}

fn collider_group_names(setup: &SpringBoneSetup, chain: &SpringChain) -> Vec<String> {
    chain
        .collider_group_ids
        .iter()
        .filter_map(|group_id| {
            setup
                .collider_groups
                .iter()
                .find(|group| group.id == *group_id)
                .map(|group| group.name.clone())
        })
        .collect()
}

/// The VRM tail joint, on the bone the engine swings the last joint towards (`tail_bone_id`, else the
/// first child in bone order); a leaf last bone gets none and the consumer extends it.
fn build_tail_joint(
    chain: &SpringChain,
    last: &SidecarSpringJoint,
    bones: &[BoneInput],
) -> Option<SidecarSpringJoint> {
    let last_bone_id = chain.joints.last()?.bone_id as usize;
    bones.get(last_bone_id)?;
    let tail = match chain.tail_bone_id {
        Some(tail_bone_id) => bones.get(tail_bone_id as usize)?,
        None => bones
            .iter()
            .find(|bone| bone.parent == Some(last_bone_id))?,
    };
    Some(SidecarSpringJoint {
        bone: tail.name.clone(),
        ..last.clone()
    })
}

fn sidecar_chain_origin(origin: &ChainOrigin) -> Option<SidecarChainOrigin> {
    match origin {
        ChainOrigin::Authored => None,
        ChainOrigin::Generated { target, role } => Some(SidecarChainOrigin {
            mesh: target.mesh().to_string(),
            material: target.material().map(str::to_string),
            role: *role,
        }),
    }
}
