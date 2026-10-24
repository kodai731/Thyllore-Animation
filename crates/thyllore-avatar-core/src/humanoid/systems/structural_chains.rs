use thyllore_model_core::{BoneId, Skeleton};

use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::spring_chain::StructuralChain;

/// The chains hanging under `root`: every non-humanoid child starts one, which follows single
/// non-humanoid children until a fork or a leaf. Chains shorter than `min_joints` are left out.
pub fn find_chains_under(
    skeleton: &Skeleton,
    mapping: &HumanoidMapping,
    root: BoneId,
    min_joints: usize,
) -> Vec<StructuralChain> {
    let is_humanoid = humanoid_flags(skeleton, mapping);
    let Some(root_bone) = skeleton.get_bone(root) else {
        return Vec::new();
    };

    root_bone
        .children
        .iter()
        .filter(|&&child| !is_humanoid[child as usize])
        .map(|&child| StructuralChain {
            joints: follow_single_children(skeleton, &is_humanoid, child),
        })
        .filter(|chain| chain.joints.len() >= min_joints)
        .collect()
}

fn humanoid_flags(skeleton: &Skeleton, mapping: &HumanoidMapping) -> Vec<bool> {
    let mut flags = vec![false; skeleton.bones.len()];
    for &bone_index in mapping.by_role.values() {
        if let Some(flag) = flags.get_mut(bone_index) {
            *flag = true;
        }
    }
    flags
}

fn follow_single_children(skeleton: &Skeleton, is_humanoid: &[bool], first: BoneId) -> Vec<BoneId> {
    let mut joints = vec![first];
    let mut current = first;
    loop {
        let candidates: Vec<BoneId> = skeleton.bones[current as usize]
            .children
            .iter()
            .copied()
            .filter(|&child| !is_humanoid[child as usize])
            .collect();
        let [only_child] = candidates[..] else {
            return joints;
        };
        joints.push(only_child);
        current = only_child;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humanoid::components::role::HumanoidRole;

    fn skeleton(bones: &[(&str, Option<BoneId>)]) -> Skeleton {
        let mut skeleton = Skeleton::new("test");
        for (name, parent) in bones {
            skeleton.add_bone(name, *parent);
        }
        skeleton
    }

    fn mapping(pairs: &[(HumanoidRole, usize)]) -> HumanoidMapping {
        HumanoidMapping {
            by_role: pairs.iter().copied().collect(),
        }
    }

    #[test]
    fn single_chain_under_hips() {
        let skeleton = skeleton(&[
            ("Hips", None),
            ("braid", Some(0)),
            ("braid.001", Some(1)),
            ("braid.002", Some(2)),
        ]);
        let chains = find_chains_under(&skeleton, &mapping(&[(HumanoidRole::Hips, 0)]), 0, 1);
        assert_eq!(
            chains,
            vec![StructuralChain {
                joints: vec![1, 2, 3]
            }]
        );
    }

    #[test]
    fn two_chains_under_one_root_keep_bone_order() {
        let skeleton = skeleton(&[
            ("Hips", None),
            ("right", Some(0)),
            ("right.001", Some(1)),
            ("left", Some(0)),
            ("left.001", Some(3)),
            ("left.002", Some(4)),
        ]);
        let chains = find_chains_under(&skeleton, &mapping(&[(HumanoidRole::Hips, 0)]), 0, 1);
        assert_eq!(
            chains,
            vec![
                StructuralChain { joints: vec![1, 2] },
                StructuralChain {
                    joints: vec![3, 4, 5]
                },
            ]
        );
    }

    #[test]
    fn fork_ends_the_chain_at_the_forking_bone() {
        let skeleton = skeleton(&[
            ("Hips", None),
            ("braid", Some(0)),
            ("braid.001", Some(1)),
            ("braid.002", Some(1)),
        ]);
        let chains = find_chains_under(&skeleton, &mapping(&[(HumanoidRole::Hips, 0)]), 0, 1);
        assert_eq!(chains, vec![StructuralChain { joints: vec![1] }]);
    }

    #[test]
    fn humanoid_children_are_not_chains_and_stop_a_chain() {
        let skeleton = skeleton(&[
            ("Hips", None),
            ("Spine", Some(0)),
            ("skirt", Some(0)),
            ("skirt.001", Some(2)),
            ("LeftUpperLeg", Some(3)),
        ]);
        let humanoid = mapping(&[
            (HumanoidRole::Hips, 0),
            (HumanoidRole::Spine, 1),
            (HumanoidRole::LeftUpperLeg, 4),
        ]);
        let chains = find_chains_under(&skeleton, &humanoid, 0, 1);
        assert_eq!(chains, vec![StructuralChain { joints: vec![2, 3] }]);
    }

    #[test]
    fn min_joints_drops_a_single_end_bone_but_keeps_hair() {
        let skeleton = skeleton(&[
            ("Head", None),
            ("HeadTop_End", Some(0)),
            ("hair_00", Some(0)),
            ("hair_01", Some(2)),
        ]);
        let chains = find_chains_under(&skeleton, &mapping(&[(HumanoidRole::Head, 0)]), 0, 2);
        assert_eq!(chains, vec![StructuralChain { joints: vec![2, 3] }]);
    }

    #[test]
    fn unknown_root_yields_nothing() {
        let skeleton = skeleton(&[("Hips", None)]);
        assert!(find_chains_under(&skeleton, &mapping(&[]), 7, 1).is_empty());
    }
}
