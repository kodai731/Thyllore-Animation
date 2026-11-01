use std::collections::BTreeMap;

use anyhow::Context;
use cgmath::{InnerSpace, Vector3};
use serde_json::{json, Value};

use super::force::vrm_magnitudes;
use crate::humanoid::components::skeleton_input::BoneInput;
use crate::vrchat::sidecar_spring::{
    map_sidecar_colliders, BoneRestFrame, ColliderSpace, SidecarCollider, SidecarColliderShape,
    SidecarSpring, SidecarSpringChain, SidecarSpringJoint,
};

pub const VRMC_SPRING_BONE: &str = "VRMC_springBone";

/// The sidecar spring with the skeleton it was authored on; `bones` carry rest positions in model metres,
/// `frames` map bone-local vectors to model space, and `node_of_bone` is the glTF node of each bone name.
pub struct SpringBoneInput<'a> {
    pub spring: &'a SidecarSpring,
    pub bones: &'a [BoneInput],
    pub frames: &'a [BoneRestFrame],
    pub node_of_bone: &'a BTreeMap<String, u32>,
}

/// The `VRMC_springBone` extension: colliders in node space, every enabled chain as a spring whose
/// joints carry the VRM magnitudes of their force.
pub fn build_vrmc_spring_bone(input: SpringBoneInput<'_>) -> anyhow::Result<Value> {
    let local = map_sidecar_colliders(
        input.spring,
        input.bones,
        input.frames,
        ColliderSpace::BoneLocal,
    );
    let colliders = local
        .colliders
        .iter()
        .map(|collider| build_collider(collider, input.node_of_bone))
        .collect::<anyhow::Result<Vec<Value>>>()?;

    let collider_index: BTreeMap<&str, usize> = local
        .colliders
        .iter()
        .enumerate()
        .map(|(index, collider)| (collider.name.as_str(), index))
        .collect();
    let collider_groups = local
        .collider_groups
        .iter()
        .map(|group| {
            let members = group
                .colliders
                .iter()
                .map(|name| {
                    collider_index.get(name.as_str()).copied().with_context(|| {
                        format!(
                            "collider group {:?} names unknown collider {name:?}",
                            group.name
                        )
                    })
                })
                .collect::<anyhow::Result<Vec<usize>>>()?;
            Ok(json!({ "name": group.name, "colliders": members }))
        })
        .collect::<anyhow::Result<Vec<Value>>>()?;

    let group_index: BTreeMap<&str, usize> = local
        .collider_groups
        .iter()
        .enumerate()
        .map(|(index, group)| (group.name.as_str(), index))
        .collect();
    let springs = local
        .chains
        .iter()
        .filter(|chain| chain.enabled)
        .map(|chain| build_spring(chain, &group_index, &input))
        .collect::<anyhow::Result<Vec<Value>>>()?;

    Ok(json!({
        "specVersion": "1.0",
        "colliders": colliders,
        "colliderGroups": collider_groups,
        "springs": springs,
    }))
}

fn node_of(node_of_bone: &BTreeMap<String, u32>, bone: &str) -> anyhow::Result<u32> {
    node_of_bone
        .get(bone)
        .copied()
        .with_context(|| format!("bone {bone:?} has no glTF node"))
}

fn build_collider(
    collider: &SidecarCollider,
    node_of_bone: &BTreeMap<String, u32>,
) -> anyhow::Result<Value> {
    let node = node_of(node_of_bone, &collider.bone)?;
    let shape = match (&collider.shape, collider.tail) {
        (SidecarColliderShape::Capsule, Some(tail)) => json!({
            "capsule": { "offset": collider.offset, "radius": collider.radius, "tail": tail }
        }),
        (SidecarColliderShape::Capsule, None) | (SidecarColliderShape::Sphere, _) => json!({
            "sphere": { "offset": collider.offset, "radius": collider.radius }
        }),
    };
    Ok(json!({ "node": node, "shape": shape }))
}

fn build_spring(
    chain: &SidecarSpringChain,
    group_index: &BTreeMap<&str, usize>,
    input: &SpringBoneInput<'_>,
) -> anyhow::Result<Value> {
    let strand_length = strand_length_m(&chain.joints, input.bones);
    let joints = chain
        .joints
        .iter()
        .map(|joint| build_joint(joint, strand_length, input))
        .collect::<anyhow::Result<Vec<Value>>>()?;
    let collider_groups = chain
        .collider_groups
        .iter()
        .map(|name| {
            group_index.get(name.as_str()).copied().with_context(|| {
                format!(
                    "chain {:?} names unknown collider group {name:?}",
                    chain.name
                )
            })
        })
        .collect::<anyhow::Result<Vec<usize>>>()?;

    let mut spring = json!({
        "name": chain.name,
        "joints": joints,
        "colliderGroups": collider_groups,
    });
    if let Some(center) = &chain.center {
        spring["center"] = json!(node_of(input.node_of_bone, center)?);
    }
    Ok(spring)
}

fn build_joint(
    joint: &SidecarSpringJoint,
    strand_length_m: f32,
    input: &SpringBoneInput<'_>,
) -> anyhow::Result<Value> {
    let node = node_of(input.node_of_bone, &joint.bone)?;
    let magnitudes = vrm_magnitudes(joint.force, strand_length_m);
    let hit_radius = bone_frame(input, &joint.bone)
        .map(|frame| frame.length_to_local(joint.hit_radius))
        .unwrap_or(joint.hit_radius);
    Ok(json!({
        "node": node,
        "hitRadius": hit_radius,
        "stiffness": magnitudes.stiffness,
        "gravityPower": magnitudes.gravity_power,
        "gravityDir": joint.gravity_dir,
        "dragForce": joint.drag_force,
    }))
}

fn bone_frame<'a>(input: &'a SpringBoneInput<'_>, bone: &str) -> Option<&'a BoneRestFrame> {
    let index = input
        .bones
        .iter()
        .position(|candidate| candidate.name == bone)?;
    input.frames.get(index)
}

/// The rest length of the strand in metres: the joint-to-joint distances, the appended tail included.
fn strand_length_m(joints: &[SidecarSpringJoint], bones: &[BoneInput]) -> f32 {
    let positions: Vec<Vector3<f32>> = joints
        .iter()
        .filter_map(|joint| bones.iter().find(|bone| bone.name == joint.bone))
        .map(|bone| Vector3::from(bone.rest_position))
        .collect();
    positions
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).magnitude())
        .sum()
}
