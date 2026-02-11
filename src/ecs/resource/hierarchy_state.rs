use std::collections::HashSet;

use crate::animation::BoneId;
use crate::ecs::world::Entity;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum HierarchyDisplayMode {
    #[default]
    Entities,
    Bones,
}

#[derive(Clone, Debug, Default)]
pub struct HierarchyState {
    pub selected_entity: Option<Entity>,
    pub multi_selection: HashSet<Entity>,
    pub search_filter: String,

    pub display_mode: HierarchyDisplayMode,
    pub selected_bone_id: Option<BoneId>,
    pub expanded_bone_ids: HashSet<BoneId>,
}
