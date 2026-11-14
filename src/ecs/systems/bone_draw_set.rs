use std::collections::HashSet;

use crate::animation::{BoneId, Skeleton};
use crate::ecs::component::SpringBoneSetup;
use crate::ecs::resource::gizmo::{BoneDrawMode, BoneSelectionState};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BoneDrawSet {
    drawn: Vec<bool>,
}

impl BoneDrawSet {
    pub fn from_flags(drawn: Vec<bool>) -> Self {
        Self { drawn }
    }

    pub fn all(bone_count: usize) -> Self {
        Self {
            drawn: vec![true; bone_count],
        }
    }

    pub fn is_drawn(&self, bone_index: usize) -> bool {
        self.drawn.get(bone_index).copied().unwrap_or(true)
    }

    pub fn flags(&self) -> &[bool] {
        &self.drawn
    }
}

pub fn spring_chain_bone_indices(setup: &SpringBoneSetup) -> HashSet<usize> {
    setup
        .chains
        .iter()
        .flat_map(|chain| chain.joints.iter().map(|joint| joint.bone_id as usize))
        .collect()
}

pub fn compute_bone_draw_set(
    skeleton: &Skeleton,
    mode: BoneDrawMode,
    selection: &BoneSelectionState,
    excluded: &HashSet<usize>,
) -> BoneDrawSet {
    let in_mode = bones_in_mode(skeleton, mode, selection);
    let drawn = (0..skeleton.bones.len())
        .map(|index| in_mode.contains(&index) && !excluded.contains(&index))
        .collect();
    BoneDrawSet::from_flags(drawn)
}

fn bones_in_mode(
    skeleton: &Skeleton,
    mode: BoneDrawMode,
    selection: &BoneSelectionState,
) -> HashSet<usize> {
    let selected = &selection.selected_bone_indices;
    match mode {
        BoneDrawMode::All => (0..skeleton.bones.len()).collect(),
        BoneDrawMode::None => HashSet::new(),
        BoneDrawMode::Selected => selected.clone(),
        BoneDrawMode::SelectedAndParents => with_ancestors(skeleton, selected),
        BoneDrawMode::SelectedAndChildren => with_descendants(skeleton, selected),
        BoneDrawMode::SelectedWithParentsAndChildren => {
            let mut set = with_ancestors(skeleton, selected);
            set.extend(with_descendants(skeleton, selected));
            set
        }
    }
}

fn with_ancestors(skeleton: &Skeleton, selected: &HashSet<usize>) -> HashSet<usize> {
    let mut set = selected.clone();
    for &index in selected {
        let mut parent = skeleton
            .get_bone(index as BoneId)
            .and_then(|bone| bone.parent_id);
        while let Some(parent_id) = parent {
            set.insert(parent_id as usize);
            parent = skeleton.get_bone(parent_id).and_then(|bone| bone.parent_id);
        }
    }
    set
}

fn with_descendants(skeleton: &Skeleton, selected: &HashSet<usize>) -> HashSet<usize> {
    let mut set = selected.clone();
    for &index in selected {
        for descendant in skeleton.collect_descendants(index as BoneId) {
            set.insert(descendant as usize);
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::Bone;

    fn chain_skeleton() -> Skeleton {
        let mut skeleton = Skeleton::default();
        let parents = [None, Some(0), Some(1), Some(1)];
        for (id, parent_id) in parents.iter().enumerate() {
            skeleton.bones.push(Bone {
                id: id as BoneId,
                name: format!("b{id}"),
                parent_id: *parent_id,
                children: Vec::new(),
                ..Bone::default()
            });
        }
        for id in 1..4u32 {
            let parent = skeleton.bones[id as usize].parent_id.expect("child bone");
            skeleton.bones[parent as usize].children.push(id);
        }
        skeleton
    }

    fn selecting(indices: &[usize]) -> BoneSelectionState {
        BoneSelectionState {
            selected_bone_indices: indices.iter().copied().collect(),
            active_bone_index: indices.first().copied(),
        }
    }

    #[test]
    fn all_mode_draws_every_bone_except_excluded() {
        let skeleton = chain_skeleton();
        let excluded = HashSet::from([3]);
        let set = compute_bone_draw_set(&skeleton, BoneDrawMode::All, &selecting(&[]), &excluded);
        assert_eq!(set.flags(), &[true, true, true, false]);
    }

    #[test]
    fn selected_and_parents_walks_to_the_root() {
        let skeleton = chain_skeleton();
        let set = compute_bone_draw_set(
            &skeleton,
            BoneDrawMode::SelectedAndParents,
            &selecting(&[2]),
            &HashSet::new(),
        );
        assert_eq!(set.flags(), &[true, true, true, false]);
    }

    #[test]
    fn selected_and_children_covers_the_subtree() {
        let skeleton = chain_skeleton();
        let set = compute_bone_draw_set(
            &skeleton,
            BoneDrawMode::SelectedAndChildren,
            &selecting(&[1]),
            &HashSet::new(),
        );
        assert_eq!(set.flags(), &[false, true, true, true]);
    }

    #[test]
    fn none_mode_draws_nothing_and_out_of_range_is_drawn() {
        let skeleton = chain_skeleton();
        let set = compute_bone_draw_set(
            &skeleton,
            BoneDrawMode::None,
            &selecting(&[1]),
            &HashSet::new(),
        );
        assert!(set.flags().iter().all(|drawn| !drawn));
        assert!(BoneDrawSet::default().is_drawn(7));
    }
}
