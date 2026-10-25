use cgmath::SquareMatrix;

use crate::garment::components::chain_gen::{GeneratedChain, InfluenceTarget, VertexInfluences};
use thyllore_model_core::{BoneId, Skeleton, SkinData};

pub fn append_chain_bones(
    skeleton: &mut Skeleton,
    root: BoneId,
    chains: &[GeneratedChain],
) -> anyhow::Result<Vec<Vec<BoneId>>> {
    let existing_names: std::collections::HashSet<&str> =
        skeleton.bones.iter().map(|b| b.name.as_str()).collect();

    for chain in chains {
        for joint in &chain.joints {
            if existing_names.contains(joint.name.as_str()) {
                return Err(anyhow::anyhow!("bone {} already exists", joint.name));
            }
        }
    }

    let root_bone = skeleton
        .get_bone(root)
        .ok_or_else(|| anyhow::anyhow!("root bone not found"))?;
    let b_root = root_bone
        .inverse_bind_pose
        .invert()
        .ok_or_else(|| anyhow::anyhow!("root inverse_bind_pose is singular"))?;

    let mut bone_ids: Vec<Vec<BoneId>> = Vec::with_capacity(chains.len());

    for chain in chains {
        let mut chain_bones: Vec<BoneId> = Vec::with_capacity(chain.joints.len());

        let mut parent_id: Option<BoneId> = Some(root);
        let mut b_parent = b_root;

        for joint in &chain.joints {
            let mut b_j = b_root;
            b_j.w = joint.head.extend(1.0);
            let parent_inverse = b_parent.invert().ok_or_else(|| {
                anyhow::anyhow!("bind pose of the parent of {} is singular", joint.name)
            })?;
            let joint_inverse = b_j
                .invert()
                .ok_or_else(|| anyhow::anyhow!("bind pose of {} is singular", joint.name))?;

            let bone_id = skeleton.add_bone(&joint.name, parent_id);
            let bone = skeleton.get_bone_mut(bone_id).expect("bone was just added");
            bone.local_transform = parent_inverse * b_j;
            bone.inverse_bind_pose = joint_inverse;

            parent_id = Some(bone_id);
            b_parent = b_j;
            chain_bones.push(bone_id);
        }

        bone_ids.push(chain_bones);
    }

    Ok(bone_ids)
}

pub fn write_chain_weights(
    skin: &mut SkinData,
    root: BoneId,
    bone_ids: &[Vec<BoneId>],
    influences: &[VertexInfluences],
) {
    for inf in influences {
        let mut mapped: Vec<(u32, f32)> = Vec::new();

        for (target, weight) in &inf.influences {
            let bone_idx = match target {
                InfluenceTarget::Root => root,
                InfluenceTarget::Joint { chain, joint } => bone_ids[*chain][*joint],
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
