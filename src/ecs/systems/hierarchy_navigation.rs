use crate::animation::{BoneId, Skeleton};
use crate::asset::AssetStorage;
use crate::ecs::component::EditorDisplay;
use crate::ecs::resource::{HierarchyDisplayMode, HierarchyState};
use crate::ecs::systems::{
    collapse_entity, collapse_or_ascend, expand_entity, expand_or_descend, hierarchy_collapse_bone,
    hierarchy_expand_bone, hierarchy_is_bone_expanded, hierarchy_select, hierarchy_select_bone,
    move_row, query_hierarchy_tree, sibling_rows, type_ahead_row, HierarchyEntry, TreeMove,
    TreeRow, TreeStep,
};
use crate::ecs::world::{Children, Entity, World};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TreeNavigation {
    Move(TreeMove),
    Extend(TreeMove),
    ExpandOrDescend,
    CollapseOrAscend,
    ExpandRecursive,
    CollapseRecursive,
    ExpandSiblings,
    SelectAllVisible,
    TypeAhead { character: char, now_seconds: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeSelection {
    Entity(Entity),
    Bone(BoneId),
    Unchanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SubtreeExpansion {
    Expand,
    Collapse,
}

#[derive(Clone, Debug)]
pub struct BoneRow {
    pub bone_id: BoneId,
    pub name: String,
    pub depth: usize,
    pub has_children: bool,
    pub expanded: bool,
    pub selected: bool,
}

pub fn query_bone_rows(skeleton: &Skeleton, state: &HierarchyState) -> Vec<BoneRow> {
    let mut rows = Vec::with_capacity(skeleton.bones.len());
    for &root_id in &skeleton.root_bone_ids {
        collect_bone_rows(skeleton, state, root_id, 0, &mut rows);
    }
    rows
}

fn collect_bone_rows(
    skeleton: &Skeleton,
    state: &HierarchyState,
    bone_id: BoneId,
    depth: usize,
    rows: &mut Vec<BoneRow>,
) {
    let Some(bone) = skeleton.get_bone(bone_id) else {
        return;
    };
    let expanded = hierarchy_is_bone_expanded(state, bone_id);
    rows.push(BoneRow {
        bone_id,
        name: bone.name.clone(),
        depth,
        has_children: !bone.children.is_empty(),
        expanded,
        selected: state.selected_bone_id == Some(bone_id),
    });

    if expanded {
        for &child_id in &bone.children {
            collect_bone_rows(skeleton, state, child_id, depth + 1, rows);
        }
    }
}

pub fn navigate_hierarchy(
    world: &mut World,
    assets: &AssetStorage,
    navigation: TreeNavigation,
    page_rows: usize,
) -> TreeSelection {
    let display_mode = world.resource::<HierarchyState>().display_mode;
    let selection = match display_mode {
        HierarchyDisplayMode::Entities => navigate_entities(world, navigation, page_rows),
        HierarchyDisplayMode::Bones => navigate_bones(world, assets, navigation, page_rows),
    };

    if selection != TreeSelection::Unchanged {
        world.resource_mut::<HierarchyState>().scroll_to_selected = true;
    }
    selection
}

fn navigate_entities(
    world: &mut World,
    navigation: TreeNavigation,
    page_rows: usize,
) -> TreeSelection {
    let entries = {
        let state = world.resource::<HierarchyState>();
        query_hierarchy_tree(world, &state)
    };
    let rows: Vec<TreeRow> = entries.iter().map(entity_tree_row).collect();
    let current = {
        let state = world.resource::<HierarchyState>();
        state
            .selected_entity
            .and_then(|entity| entries.iter().position(|entry| entry.entity == entity))
    };

    match navigation {
        TreeNavigation::Move(tree_move) => move_row(&rows, current, tree_move, page_rows)
            .map_or(TreeSelection::Unchanged, |index| {
                select_entity_row(world, &entries, index)
            }),
        TreeNavigation::Extend(tree_move) => move_row(&rows, current, tree_move, page_rows)
            .map_or(TreeSelection::Unchanged, |index| {
                extend_entity_selection(world, &entries, current, index)
            }),
        TreeNavigation::ExpandOrDescend => current
            .and_then(|index| expand_or_descend(&rows, index))
            .map_or(TreeSelection::Unchanged, |step| {
                apply_entity_step(world, &entries, step)
            }),
        TreeNavigation::CollapseOrAscend => current
            .and_then(|index| collapse_or_ascend(&rows, index))
            .map_or(TreeSelection::Unchanged, |step| {
                apply_entity_step(world, &entries, step)
            }),
        TreeNavigation::ExpandRecursive => {
            if let Some(index) = current {
                set_subtree_expanded(world, entries[index].entity, SubtreeExpansion::Expand);
            }
            TreeSelection::Unchanged
        }
        TreeNavigation::CollapseRecursive => {
            if let Some(index) = current {
                set_subtree_expanded(world, entries[index].entity, SubtreeExpansion::Collapse);
            }
            TreeSelection::Unchanged
        }
        TreeNavigation::ExpandSiblings => {
            if let Some(index) = current {
                for sibling in sibling_rows(&rows, index) {
                    if rows[sibling].has_children {
                        expand_entity(world, entries[sibling].entity);
                    }
                }
            }
            TreeSelection::Unchanged
        }
        TreeNavigation::SelectAllVisible => select_all_entities(world, &entries),
        TreeNavigation::TypeAhead {
            character,
            now_seconds,
        } => {
            let prefix = {
                let mut state = world.resource_mut::<HierarchyState>();
                state.type_ahead.push(character, now_seconds).to_string()
            };
            type_ahead_row(
                entries.iter().map(|entry| entry.name.as_str()),
                current,
                &prefix,
            )
            .map_or(TreeSelection::Unchanged, |index| {
                select_entity_row(world, &entries, index)
            })
        }
    }
}

fn entity_tree_row(entry: &HierarchyEntry) -> TreeRow {
    TreeRow {
        depth: entry.depth,
        has_children: entry.has_children,
        expanded: entry.expanded,
    }
}

fn select_entity_row(world: &World, entries: &[HierarchyEntry], index: usize) -> TreeSelection {
    let entity = entries[index].entity;
    let mut state = world.resource_mut::<HierarchyState>();
    hierarchy_select(&mut state, entity);
    TreeSelection::Entity(entity)
}

fn extend_entity_selection(
    world: &World,
    entries: &[HierarchyEntry],
    current: Option<usize>,
    target: usize,
) -> TreeSelection {
    let mut state = world.resource_mut::<HierarchyState>();
    let anchor = state
        .selection_anchor
        .and_then(|anchor| entries.iter().position(|entry| entry.entity == anchor))
        .or(current)
        .unwrap_or(target);

    let (low, high) = (anchor.min(target), anchor.max(target));
    state.multi_selection = entries[low..=high]
        .iter()
        .map(|entry| entry.entity)
        .collect();
    state.selected_entity = Some(entries[target].entity);
    state.selection_anchor = Some(entries[anchor].entity);
    TreeSelection::Entity(entries[target].entity)
}

fn apply_entity_step(
    world: &mut World,
    entries: &[HierarchyEntry],
    step: TreeStep,
) -> TreeSelection {
    match step {
        TreeStep::Select(index) => select_entity_row(world, entries, index),
        TreeStep::Expand(index) => {
            expand_entity(world, entries[index].entity);
            TreeSelection::Unchanged
        }
        TreeStep::Collapse(index) => {
            collapse_entity(world, entries[index].entity);
            TreeSelection::Unchanged
        }
    }
}

fn set_subtree_expanded(world: &mut World, root: Entity, expansion: SubtreeExpansion) {
    let expanded = expansion == SubtreeExpansion::Expand;
    let mut pending = vec![root];
    while let Some(entity) = pending.pop() {
        if let Some(children) = world.get_component::<Children>(entity) {
            pending.extend(children.0.iter().copied());
        }
        if let Some(display) = world.get_component_mut::<EditorDisplay>(entity) {
            display.expanded = expanded;
        }
    }
}

fn select_all_entities(world: &World, entries: &[HierarchyEntry]) -> TreeSelection {
    let Some(first) = entries.first() else {
        return TreeSelection::Unchanged;
    };
    let mut state = world.resource_mut::<HierarchyState>();
    let active = state
        .selected_entity
        .filter(|entity| entries.iter().any(|entry| entry.entity == *entity))
        .unwrap_or(first.entity);
    state.multi_selection = entries.iter().map(|entry| entry.entity).collect();
    state.selected_entity = Some(active);
    TreeSelection::Entity(active)
}

fn navigate_bones(
    world: &mut World,
    assets: &AssetStorage,
    navigation: TreeNavigation,
    page_rows: usize,
) -> TreeSelection {
    let Some(skeleton) = assets
        .skeletons
        .values()
        .next()
        .map(|asset| &asset.skeleton)
    else {
        return TreeSelection::Unchanged;
    };
    let bone_rows = {
        let state = world.resource::<HierarchyState>();
        query_bone_rows(skeleton, &state)
    };
    let rows: Vec<TreeRow> = bone_rows.iter().map(bone_tree_row).collect();
    let current = {
        let state = world.resource::<HierarchyState>();
        state
            .selected_bone_id
            .and_then(|bone_id| bone_rows.iter().position(|row| row.bone_id == bone_id))
    };

    match navigation {
        TreeNavigation::Move(tree_move) | TreeNavigation::Extend(tree_move) => {
            move_row(&rows, current, tree_move, page_rows)
                .map_or(TreeSelection::Unchanged, |index| {
                    select_bone_row(world, &bone_rows, index)
                })
        }
        TreeNavigation::ExpandOrDescend => current
            .and_then(|index| expand_or_descend(&rows, index))
            .map_or(TreeSelection::Unchanged, |step| {
                apply_bone_step(world, &bone_rows, step)
            }),
        TreeNavigation::CollapseOrAscend => current
            .and_then(|index| collapse_or_ascend(&rows, index))
            .map_or(TreeSelection::Unchanged, |step| {
                apply_bone_step(world, &bone_rows, step)
            }),
        TreeNavigation::ExpandRecursive => {
            if let Some(index) = current {
                let mut state = world.resource_mut::<HierarchyState>();
                let root = bone_rows[index].bone_id;
                hierarchy_expand_bone(&mut state, root);
                for descendant in skeleton.collect_descendants(root) {
                    hierarchy_expand_bone(&mut state, descendant);
                }
            }
            TreeSelection::Unchanged
        }
        TreeNavigation::CollapseRecursive => {
            if let Some(index) = current {
                let mut state = world.resource_mut::<HierarchyState>();
                let root = bone_rows[index].bone_id;
                hierarchy_collapse_bone(&mut state, root);
                for descendant in skeleton.collect_descendants(root) {
                    hierarchy_collapse_bone(&mut state, descendant);
                }
            }
            TreeSelection::Unchanged
        }
        TreeNavigation::ExpandSiblings => {
            if let Some(index) = current {
                let mut state = world.resource_mut::<HierarchyState>();
                for sibling in sibling_rows(&rows, index) {
                    if rows[sibling].has_children {
                        hierarchy_expand_bone(&mut state, bone_rows[sibling].bone_id);
                    }
                }
            }
            TreeSelection::Unchanged
        }
        TreeNavigation::SelectAllVisible => TreeSelection::Unchanged,
        TreeNavigation::TypeAhead {
            character,
            now_seconds,
        } => {
            let prefix = {
                let mut state = world.resource_mut::<HierarchyState>();
                state.type_ahead.push(character, now_seconds).to_string()
            };
            type_ahead_row(
                bone_rows.iter().map(|row| row.name.as_str()),
                current,
                &prefix,
            )
            .map_or(TreeSelection::Unchanged, |index| {
                select_bone_row(world, &bone_rows, index)
            })
        }
    }
}

fn bone_tree_row(row: &BoneRow) -> TreeRow {
    TreeRow {
        depth: row.depth,
        has_children: row.has_children,
        expanded: row.expanded,
    }
}

fn select_bone_row(world: &World, rows: &[BoneRow], index: usize) -> TreeSelection {
    let bone_id = rows[index].bone_id;
    hierarchy_select_bone(&mut world.resource_mut::<HierarchyState>(), bone_id);
    TreeSelection::Bone(bone_id)
}

fn apply_bone_step(world: &World, rows: &[BoneRow], step: TreeStep) -> TreeSelection {
    match step {
        TreeStep::Select(index) => select_bone_row(world, rows, index),
        TreeStep::Expand(index) => {
            hierarchy_expand_bone(
                &mut world.resource_mut::<HierarchyState>(),
                rows[index].bone_id,
            );
            TreeSelection::Unchanged
        }
        TreeStep::Collapse(index) => {
            hierarchy_collapse_bone(
                &mut world.resource_mut::<HierarchyState>(),
                rows[index].bone_id,
            );
            TreeSelection::Unchanged
        }
    }
}
