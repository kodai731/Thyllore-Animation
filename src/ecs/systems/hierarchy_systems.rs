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

    let selected = state.is_selected(entity);

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
