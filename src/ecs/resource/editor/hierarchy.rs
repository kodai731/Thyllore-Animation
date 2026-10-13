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
pub struct TypeAheadBuffer {
    pub text: String,
    pub last_input_seconds: f64,
}

impl TypeAheadBuffer {
    pub const RESET_AFTER_SECONDS: f64 = 1.0;

    pub fn push(&mut self, character: char, now_seconds: f64) -> &str {
        if now_seconds - self.last_input_seconds > Self::RESET_AFTER_SECONDS {
            self.text.clear();
        }
        self.text.push(character);
        self.last_input_seconds = now_seconds;
        &self.text
    }
}

#[derive(Clone, Debug, Default)]
pub struct HierarchyState {
    pub selected_entity: Option<Entity>,
    pub multi_selection: HashSet<Entity>,
    pub selection_anchor: Option<Entity>,
    pub search_filter: String,
    pub scroll_to_selected: bool,
    pub type_ahead: TypeAheadBuffer,

    pub display_mode: HierarchyDisplayMode,
    pub selected_bone_id: Option<BoneId>,
    pub expanded_bone_ids: HashSet<BoneId>,
}
