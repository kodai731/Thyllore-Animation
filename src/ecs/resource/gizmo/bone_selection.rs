use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct BoneSelectionState {
    pub selected_bone_indices: HashSet<usize>,
    pub active_bone_index: Option<usize>,
}

impl Default for BoneSelectionState {
    fn default() -> Self {
        Self {
            selected_bone_indices: HashSet::new(),
            active_bone_index: None,
        }
    }
}
