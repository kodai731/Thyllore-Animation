use std::collections::HashSet;

use crate::ecs::world::Entity;

#[derive(Clone, Debug, Default)]
pub struct HierarchyState {
    pub selected_entity: Option<Entity>,
    pub multi_selection: HashSet<Entity>,
    pub search_filter: String,
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
}
