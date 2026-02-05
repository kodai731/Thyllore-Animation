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

impl HierarchyState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select(&mut self, entity: Entity) {
        self.selected_entity = Some(entity);
        self.multi_selection.clear();
        self.multi_selection.insert(entity);
    }

    pub fn deselect_all(&mut self) {
        self.selected_entity = None;
        self.multi_selection.clear();
    }

    pub fn toggle_selection(&mut self, entity: Entity) {
        if self.multi_selection.contains(&entity) {
            self.multi_selection.remove(&entity);
            if self.selected_entity == Some(entity) {
                self.selected_entity = self.multi_selection.iter().next().copied();
            }
        } else {
            self.multi_selection.insert(entity);
            if self.selected_entity.is_none() {
                self.selected_entity = Some(entity);
            }
        }
    }

    pub fn is_selected(&self, entity: Entity) -> bool {
        self.multi_selection.contains(&entity)
    }

    pub fn select_bone(&mut self, bone_id: BoneId) {
        self.selected_bone_id = Some(bone_id);
    }

    pub fn deselect_bone(&mut self) {
        self.selected_bone_id = None;
    }

    pub fn expand_bone(&mut self, bone_id: BoneId) {
        self.expanded_bone_ids.insert(bone_id);
    }

    pub fn collapse_bone(&mut self, bone_id: BoneId) {
        self.expanded_bone_ids.remove(&bone_id);
    }

    pub fn is_bone_expanded(&self, bone_id: BoneId) -> bool {
        self.expanded_bone_ids.contains(&bone_id)
    }
}
