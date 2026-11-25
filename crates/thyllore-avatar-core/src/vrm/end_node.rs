use anyhow::{bail, Context};
use glam::Vec3;

use crate::cloth_setup::components::chain_gen::end_bone_name;
use crate::humanoid::components::skeleton_input::BoneInput;
use crate::vrchat::sidecar_spring::{BoneRestFrame, SidecarSpring, SidecarSpringChain};

/// A node to add under the leaf bone an enabled chain ends on, so the VRM spring gets the terminal
/// joint the format requires; `translation` is in the leaf bone's local units.
#[derive(Clone, Debug, PartialEq)]
pub struct SpringEndNode {
    pub chain: String,
    pub parent_bone: String,
    pub name: String,
    pub translation: [f32; 3],
}

/// The end nodes the VRM writer must add before building `VRMC_springBone`: one per enabled chain
/// without a tail whose last joint bone has no child, continuing the last segment past the leaf.
pub fn spring_end_nodes(
    spring: &SidecarSpring,
    bones: &[BoneInput],
    frames: &[BoneRestFrame],
) -> anyhow::Result<Vec<SpringEndNode>> {
    spring
        .chains
        .iter()
        .filter(|chain| chain.enabled && chain.tail.is_none())
        .filter_map(|chain| end_node_of_chain(chain, bones, frames).transpose())
        .collect()
}

fn end_node_of_chain(
    chain: &SidecarSpringChain,
    bones: &[BoneInput],
    frames: &[BoneRestFrame],
) -> anyhow::Result<Option<SpringEndNode>> {
    let last = chain
        .joints
        .last()
        .with_context(|| format!("chain {:?} has no joints", chain.name))?;
    let leaf_index = bones
        .iter()
        .position(|bone| bone.name == last.bone)
        .with_context(|| {
            format!(
                "chain {:?} ends on unknown bone {:?}",
                chain.name, last.bone
            )
        })?;
    if bones.iter().any(|bone| bone.parent == Some(leaf_index)) {
        return Ok(None);
    }

    let leaf = &bones[leaf_index];
    let parent = leaf
        .parent
        .and_then(|index| bones.get(index))
        .with_context(|| {
            format!(
                "chain {:?} ends on root bone {:?}, which gives no direction for an end node",
                chain.name, leaf.name
            )
        })?;
    let frame = frames
        .get(leaf_index)
        .with_context(|| format!("bone {:?} has no rest frame", leaf.name))?;
    let name = end_bone_name(&leaf.name);
    if bones.iter().any(|bone| bone.name == name) {
        bail!(
            "end node {name:?} for chain {:?} collides with an existing bone",
            chain.name
        );
    }

    let last_segment = Vec3::from(leaf.rest_position) - Vec3::from(parent.rest_position);
    Ok(Some(SpringEndNode {
        chain: chain.name.clone(),
        parent_bone: leaf.name.clone(),
        name,
        translation: frame.to_local(last_segment).into(),
    }))
}
