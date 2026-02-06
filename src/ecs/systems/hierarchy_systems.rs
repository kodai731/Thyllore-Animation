use crate::animation::BoneId;
use crate::ecs::component::EditorDisplay;
use crate::ecs::resource::HierarchyState;
use crate::ecs::world::{Children, Entity, Name, World};

#[derive(Clone, Debug)]
pub struct HierarchyEntry {
    pub entity: Entity,
    pub name: String,
    pub icon_char: char,
    pub depth: usize,
    pub has_children: bool,
    pub expanded: bool,
    pub selected: bool,
}

pub fn query_hierarchy_tree(world: &World, state: &HierarchyState) -> Vec<HierarchyEntry> {
    let mut entries = Vec::new();
    let root_entities = world.get_root_entities();

    for entity in root_entities {
        collect_hierarchy_entries(world, state, entity, 0, &mut entries);
    }

    if !state.search_filter.is_empty() {
        let filter_lower = state.search_filter.to_lowercase();
        entries.retain(|entry| entry.name.to_lowercase().contains(&filter_lower));
    }

    entries
}

fn collect_hierarchy_entries(
    world: &World,
    state: &HierarchyState,
    entity: Entity,
    depth: usize,
    entries: &mut Vec<HierarchyEntry>,
) {
    let name = world
        .get_component::<Name>(entity)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| format!("Entity {}", entity));

    let editor_display = world.get_component::<EditorDisplay>(entity);
    let icon_char = editor_display
        .map(|ed| ed.icon.to_char())
        .unwrap_or(' ');

    let expanded = editor_display.map(|ed| ed.expanded).unwrap_or(false);

    let children = world.get_component::<Children>(entity);
    let has_children = children.map(|c| !c.0.is_empty()).unwrap_or(false);

    let selected = hierarchy_is_selected(state, entity);

    entries.push(HierarchyEntry {
        entity,
        name,
        icon_char,
        depth,
        has_children,
        expanded,
        selected,
    });

    if expanded {
        if let Some(children) = children {
            for &child in &children.0 {
                collect_hierarchy_entries(world, state, child, depth + 1, entries);
            }
        }
    }
}

pub fn toggle_entity_expand(world: &mut World, entity: Entity) {
    if let Some(editor_display) = world.get_component_mut::<EditorDisplay>(entity) {
        editor_display.expanded = !editor_display.expanded;
    }
}

pub fn expand_entity(world: &mut World, entity: Entity) {
    if let Some(editor_display) = world.get_component_mut::<EditorDisplay>(entity) {
        editor_display.expanded = true;
    }
}

pub fn collapse_entity(world: &mut World, entity: Entity) {
    if let Some(editor_display) = world.get_component_mut::<EditorDisplay>(entity) {
        editor_display.expanded = false;
    }
}

pub fn hierarchy_select(state: &mut HierarchyState, entity: Entity) {
    state.selected_entity = Some(entity);
    state.multi_selection.clear();
    state.multi_selection.insert(entity);
}

pub fn hierarchy_deselect_all(state: &mut HierarchyState) {
    state.selected_entity = None;
    state.multi_selection.clear();
}

pub fn hierarchy_toggle_selection(state: &mut HierarchyState, entity: Entity) {
    if state.multi_selection.contains(&entity) {
        state.multi_selection.remove(&entity);
        if state.selected_entity == Some(entity) {
            state.selected_entity = state.multi_selection.iter().next().copied();
        }
    } else {
        state.multi_selection.insert(entity);
        if state.selected_entity.is_none() {
            state.selected_entity = Some(entity);
        }
    }
}

pub fn hierarchy_is_selected(state: &HierarchyState, entity: Entity) -> bool {
    state.multi_selection.contains(&entity)
}

pub fn hierarchy_select_bone(state: &mut HierarchyState, bone_id: BoneId) {
    state.selected_bone_id = Some(bone_id);
}

pub fn hierarchy_deselect_bone(state: &mut HierarchyState) {
    state.selected_bone_id = None;
}

pub fn hierarchy_expand_bone(state: &mut HierarchyState, bone_id: BoneId) {
    state.expanded_bone_ids.insert(bone_id);
}

pub fn hierarchy_collapse_bone(state: &mut HierarchyState, bone_id: BoneId) {
    state.expanded_bone_ids.remove(&bone_id);
}

pub fn hierarchy_is_bone_expanded(state: &HierarchyState, bone_id: BoneId) -> bool {
    state.expanded_bone_ids.contains(&bone_id)
}
