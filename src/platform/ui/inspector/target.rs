use crate::animation::BoneId;
use crate::ecs::resource::{HierarchyDisplayMode, HierarchyState};
use crate::ecs::world::Entity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectorTarget {
    Entity(Entity),
    Bone(BoneId),
}

pub fn inspector_target(state: &HierarchyState) -> Option<InspectorTarget> {
    match (
        state.display_mode,
        state.selected_bone_id,
        state.selected_entity,
    ) {
        (HierarchyDisplayMode::Bones, Some(bone_id), _) => Some(InspectorTarget::Bone(bone_id)),
        (_, _, Some(entity)) => Some(InspectorTarget::Entity(entity)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(
        display_mode: HierarchyDisplayMode,
        bone: Option<BoneId>,
        entity: Option<Entity>,
    ) -> HierarchyState {
        HierarchyState {
            display_mode,
            selected_bone_id: bone,
            selected_entity: entity,
            ..HierarchyState::default()
        }
    }

    #[test]
    fn bones_tab_with_a_bone_targets_the_bone() {
        let state = state(HierarchyDisplayMode::Bones, Some(4), Some(1));
        assert_eq!(inspector_target(&state), Some(InspectorTarget::Bone(4)));
    }

    #[test]
    fn entities_tab_targets_the_entity_even_with_a_bone() {
        let state = state(HierarchyDisplayMode::Entities, Some(4), Some(1));
        assert_eq!(inspector_target(&state), Some(InspectorTarget::Entity(1)));
    }

    #[test]
    fn bones_tab_without_a_bone_falls_back_to_the_entity() {
        let state = state(HierarchyDisplayMode::Bones, None, Some(1));
        assert_eq!(inspector_target(&state), Some(InspectorTarget::Entity(1)));
    }

    #[test]
    fn nothing_selected_targets_nothing() {
        let state = state(HierarchyDisplayMode::Bones, None, None);
        assert_eq!(inspector_target(&state), None);
    }
}
