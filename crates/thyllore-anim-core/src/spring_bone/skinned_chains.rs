use std::collections::HashSet;

use thyllore_model_core::SkinData;

use super::SpringBoneSetup;

/// Joint bones of every chain that moves at least one vertex of the skin, in bone order.
pub fn chain_joints_skinned_to(setup: &SpringBoneSetup, skin: &SkinData) -> Vec<usize> {
    let influencing: HashSet<usize> = skin
        .bone_indices
        .iter()
        .zip(&skin.bone_weights)
        .flat_map(|(indices, weights)| {
            (0..4).filter_map(move |slot| (weights[slot] > 0.0).then(|| indices[slot] as usize))
        })
        .collect();

    let mut joints: Vec<usize> = setup
        .chains
        .iter()
        .filter(|chain| {
            chain
                .joints
                .iter()
                .any(|joint| influencing.contains(&(joint.bone_id as usize)))
        })
        .flat_map(|chain| chain.joints.iter().map(|joint| joint.bone_id as usize))
        .collect();
    joints.sort_unstable();
    joints.dedup();
    joints
}

#[cfg(test)]
mod tests {
    use cgmath::{Vector4, Zero};

    use super::*;
    use crate::spring_bone::{setup::SpringJointParam, SpringChain};

    fn chain(id: u32, bone_ids: &[u32]) -> SpringChain {
        SpringChain {
            id,
            joints: bone_ids
                .iter()
                .map(|&bone_id| SpringJointParam {
                    bone_id,
                    ..SpringJointParam::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    fn skin(vertices: &[(Vector4<u32>, Vector4<f32>)]) -> SkinData {
        SkinData {
            bone_indices: vertices.iter().map(|(i, _)| *i).collect(),
            bone_weights: vertices.iter().map(|(_, w)| *w).collect(),
            ..SkinData::default()
        }
    }

    #[test]
    fn skinned_chains_take_every_joint_of_a_chain_touched_by_one_weight() {
        let setup = SpringBoneSetup {
            chains: vec![chain(1, &[10, 11, 12]), chain(2, &[20, 21])],
            ..Default::default()
        };
        let skin = skin(&[
            (Vector4::new(5, 12, 0, 0), Vector4::new(0.7, 0.3, 0.0, 0.0)),
            (Vector4::new(5, 0, 0, 0), Vector4::new(1.0, 0.0, 0.0, 0.0)),
        ]);
        assert_eq!(chain_joints_skinned_to(&setup, &skin), vec![10, 11, 12]);
    }

    #[test]
    fn skinned_chains_ignore_zero_weight_slots() {
        let setup = SpringBoneSetup {
            chains: vec![chain(1, &[10, 11])],
            ..Default::default()
        };
        let skin = skin(&[(Vector4::new(10, 11, 0, 0), Vector4::zero())]);
        assert!(chain_joints_skinned_to(&setup, &skin).is_empty());
    }
}
