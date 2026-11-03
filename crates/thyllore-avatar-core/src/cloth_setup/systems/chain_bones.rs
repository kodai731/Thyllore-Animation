use anyhow::Context;
use cgmath::{Matrix4, SquareMatrix};
use thyllore_model_core::{BoneId, Skeleton, SkinData};

use crate::cloth_setup::components::chain_gen::{
    end_bone_name, GeneratedChain, InfluenceTarget, VertexInfluences,
};

/// The bones one generated chain adds to the skeleton: its joints root to tip and the end bone at
/// the chain's tail, which the last joint swings towards and no vertex is weighted to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainBoneIds {
    pub joints: Vec<BoneId>,
    pub end: BoneId,
}

pub fn append_chain_bones(
    skeleton: &mut Skeleton,
    root: BoneId,
    chains: &[GeneratedChain],
) -> anyhow::Result<Vec<ChainBoneIds>> {
    let existing_names: std::collections::HashSet<&str> =
        skeleton.bones.iter().map(|b| b.name.as_str()).collect();
    for name in chains.iter().flat_map(chain_bone_names) {
        if existing_names.contains(name.as_str()) {
            return Err(anyhow::anyhow!("bone {} already exists", name));
        }
    }

    let root_bone = skeleton
        .get_bone(root)
        .ok_or_else(|| anyhow::anyhow!("root bone not found"))?;
    let b_root = root_bone
        .inverse_bind_pose
        .invert()
        .ok_or_else(|| anyhow::anyhow!("root inverse_bind_pose is singular"))?;

    chains
        .iter()
        .map(|chain| {
            let mut parent = (root, b_root);
            let mut joints = Vec::with_capacity(chain.joints.len());
            for joint in &chain.joints {
                let bone_id = add_bone_at(skeleton, &joint.name, parent, joint.head, b_root)?;
                joints.push(bone_id);
                parent = (bone_id, bind_pose_at(b_root, joint.head));
            }
            let last = chain
                .joints
                .last()
                .ok_or_else(|| anyhow::anyhow!("a generated chain has no joints"))?;
            let end = add_bone_at(
                skeleton,
                &end_bone_name(&last.name),
                parent,
                chain.tail,
                b_root,
            )?;
            Ok(ChainBoneIds { joints, end })
        })
        .collect()
}

fn chain_bone_names(chain: &GeneratedChain) -> Vec<String> {
    let mut names: Vec<String> = chain
        .joints
        .iter()
        .map(|joint| joint.name.clone())
        .collect();
    if let Some(last) = chain.joints.last() {
        names.push(end_bone_name(&last.name));
    }
    names
}

fn bind_pose_at(b_root: Matrix4<f32>, head: cgmath::Vector3<f32>) -> Matrix4<f32> {
    let mut bind_pose = b_root;
    bind_pose.w = head.extend(1.0);
    bind_pose
}

/// Adds a bone with the root's orientation at `head`, under `parent` (its id and bind pose).
fn add_bone_at(
    skeleton: &mut Skeleton,
    name: &str,
    parent: (BoneId, Matrix4<f32>),
    head: cgmath::Vector3<f32>,
    b_root: Matrix4<f32>,
) -> anyhow::Result<BoneId> {
    let (parent_id, b_parent) = parent;
    let b_bone = bind_pose_at(b_root, head);
    let parent_inverse = b_parent
        .invert()
        .with_context(|| format!("bind pose of the parent of {name} is singular"))?;
    let bone_inverse = b_bone
        .invert()
        .with_context(|| format!("bind pose of {name} is singular"))?;

    let bone_id = skeleton.add_bone(name, Some(parent_id));
    let bone = skeleton.get_bone_mut(bone_id).expect("bone was just added");
    bone.local_transform = parent_inverse * b_bone;
    bone.inverse_bind_pose = bone_inverse;
    Ok(bone_id)
}

pub fn write_chain_weights(
    skin: &mut SkinData,
    root: BoneId,
    bone_ids: &[ChainBoneIds],
    influences: &[VertexInfluences],
) {
    for inf in influences {
        let mut mapped: Vec<(u32, f32)> = Vec::new();

        for (target, weight) in &inf.influences {
            let bone_idx = match target {
                InfluenceTarget::Root => root,
                InfluenceTarget::Joint { chain, joint } => bone_ids[*chain].joints[*joint],
                InfluenceTarget::Bone(bone) => *bone,
            };
            mapped.push((bone_idx, *weight));
        }

        mapped.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut indices = [0u32; 4];
        let mut weights = [0.0f32; 4];
        let mut total_weight = 0.0;

        for (i, &(bone_idx, weight)) in mapped.iter().take(4).enumerate() {
            indices[i] = bone_idx;
            weights[i] = weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            for w in &mut weights {
                *w /= total_weight;
            }
        }

        let vert_idx = inf.vertex as usize;
        if vert_idx < skin.bone_indices.len() {
            skin.bone_indices[vert_idx] =
                cgmath::Vector4::new(indices[0], indices[1], indices[2], indices[3]);
            skin.bone_weights[vert_idx] =
                cgmath::Vector4::new(weights[0], weights[1], weights[2], weights[3]);
        }
    }
}

#[cfg(test)]
mod tests;
