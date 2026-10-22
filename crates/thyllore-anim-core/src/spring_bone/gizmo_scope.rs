use std::collections::HashSet;

use crate::{BoneId, Skeleton};

use super::{chains_under_bone, setup::SpringChain, SpringBoneSetup, SpringChainId};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpringChainGizmoMode {
    #[default]
    Selected,
    All,
    None,
}

impl SpringChainGizmoMode {
    pub const ALL: [SpringChainGizmoMode; 3] = [
        SpringChainGizmoMode::Selected,
        SpringChainGizmoMode::All,
        SpringChainGizmoMode::None,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            SpringChainGizmoMode::Selected => "Selected",
            SpringChainGizmoMode::All => "All",
            SpringChainGizmoMode::None => "None",
        }
    }
}

/// The chains holding the active bone: the only ones that show their joint hit radii.
pub fn chains_with_radius(
    setup: &SpringBoneSetup,
    active: Option<usize>,
    mode: SpringChainGizmoMode,
) -> Vec<SpringChainId> {
    let Some(active) = active else {
        return Vec::new();
    };
    if mode == SpringChainGizmoMode::None {
        return Vec::new();
    }
    let active_only = HashSet::from([active]);
    setup
        .chains
        .iter()
        .filter(|chain| chain_has_selected_joint(chain, &active_only))
        .map(|chain| chain.id)
        .collect()
}

/// Chains whose joint is selected, plus every chain hanging directly under a selected bone.
pub fn chains_to_draw(
    setup: &SpringBoneSetup,
    skeleton: &Skeleton,
    selected: &HashSet<usize>,
    mode: SpringChainGizmoMode,
) -> Vec<SpringChainId> {
    match mode {
        SpringChainGizmoMode::None => Vec::new(),
        SpringChainGizmoMode::All => setup.chains.iter().map(|chain| chain.id).collect(),
        SpringChainGizmoMode::Selected => {
            let mut ids: HashSet<SpringChainId> = setup
                .chains
                .iter()
                .filter(|chain| chain_has_selected_joint(chain, selected))
                .map(|chain| chain.id)
                .collect();
            for &bone_index in selected {
                ids.extend(chains_under_bone(setup, skeleton, bone_index as BoneId));
            }
            let mut ordered: Vec<SpringChainId> = ids.into_iter().collect();
            ordered.sort_unstable();
            ordered
        }
    }
}

fn chain_has_selected_joint(chain: &SpringChain, selected: &HashSet<usize>) -> bool {
    chain
        .joints
        .iter()
        .any(|joint| selected.contains(&(joint.bone_id as usize)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spring_bone::{setup::SpringJointParam, SpringChain};
    use crate::Bone;

    fn skeleton_with_two_chains() -> (Skeleton, SpringBoneSetup) {
        let mut skeleton = Skeleton::default();
        let parents = [None, Some(0), Some(1), Some(1), Some(3)];
        for (id, parent_id) in parents.iter().enumerate() {
            skeleton.bones.push(Bone {
                id: id as BoneId,
                parent_id: *parent_id,
                ..Bone::default()
            });
        }
        for id in 1..5u32 {
            let parent = skeleton.bones[id as usize].parent_id.expect("child bone");
            skeleton.bones[parent as usize].children.push(id);
        }

        let chain = |id: SpringChainId, bone_ids: &[BoneId]| SpringChain {
            id,
            joints: bone_ids
                .iter()
                .map(|&bone_id| SpringJointParam {
                    bone_id,
                    ..SpringJointParam::default()
                })
                .collect(),
            ..Default::default()
        };
        let setup = SpringBoneSetup {
            chains: vec![chain(10, &[2]), chain(11, &[3, 4])],
            ..Default::default()
        };
        (skeleton, setup)
    }

    fn selected(indices: &[usize]) -> HashSet<usize> {
        indices.iter().copied().collect()
    }

    #[test]
    fn gizmo_scope_selected_mode_draws_chains_under_the_selected_bone() {
        let (skeleton, setup) = skeleton_with_two_chains();
        let ids = chains_to_draw(
            &setup,
            &skeleton,
            &selected(&[1]),
            SpringChainGizmoMode::Selected,
        );
        assert_eq!(ids, vec![10, 11]);
    }

    #[test]
    fn gizmo_scope_selected_mode_draws_the_chain_of_a_selected_joint() {
        let (skeleton, setup) = skeleton_with_two_chains();
        let ids = chains_to_draw(
            &setup,
            &skeleton,
            &selected(&[4]),
            SpringChainGizmoMode::Selected,
        );
        assert_eq!(ids, vec![11]);
    }

    #[test]
    fn gizmo_scope_radii_follow_the_active_bone_only() {
        let (_, setup) = skeleton_with_two_chains();
        assert!(chains_with_radius(&setup, Some(1), SpringChainGizmoMode::Selected).is_empty());
        assert_eq!(
            chains_with_radius(&setup, Some(4), SpringChainGizmoMode::All),
            vec![11]
        );
    }

    #[test]
    fn gizmo_scope_all_and_none_ignore_the_selection() {
        let (skeleton, setup) = skeleton_with_two_chains();
        let empty: HashSet<usize> = HashSet::new();
        assert_eq!(
            chains_to_draw(&setup, &skeleton, &empty, SpringChainGizmoMode::All),
            vec![10, 11]
        );
        assert!(chains_to_draw(&setup, &skeleton, &empty, SpringChainGizmoMode::None).is_empty());
        assert!(
            chains_to_draw(&setup, &skeleton, &empty, SpringChainGizmoMode::Selected).is_empty()
        );
    }
}
