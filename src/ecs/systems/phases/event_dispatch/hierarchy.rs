use cgmath::Vector3;

use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::gizmo::BoneDisplayStyle;
use crate::ecs::resource::gizmo::{BoneGizmoData, BoneSelectionState};
use crate::ecs::resource::CurveEditorState;
use crate::ecs::resource::HierarchyDisplayMode;
use crate::ecs::resource::{
    ClipLibrary, EntityRemovalCommand, EntityRemovalQueue, HierarchyState, TimelineState,
};
use crate::ecs::systems::{
    collapse_entity, expand_entity, hierarchy_collapse_bone, hierarchy_deselect_all,
    hierarchy_deselect_bone, hierarchy_expand_bone, hierarchy_select, hierarchy_select_bone,
    hierarchy_toggle_selection, navigate_hierarchy, rename_entity, resolve_mesh_bone_id,
    resolve_transform_entity, update_entity_scale, update_entity_translation,
    update_entity_visible, TreeNavigation, TreeSelection,
};
use crate::ecs::world::Visibility;
use crate::ecs::world::{Children, Entity, Transform, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use cgmath::Quaternion;

#[derive(Clone, Debug)]
pub enum HierarchyEvent {
    SelectEntity(Entity),
    DeselectAll,
    ToggleEntitySelection(Entity),
    DeleteSelectedEntities,
    ExpandEntity(Entity),
    CollapseEntity(Entity),
    SetSearchFilter(String),
    SetHierarchyDisplayMode(HierarchyDisplayMode),
    SelectBone(BoneId),
    DeselectBone,
    ExpandBone(BoneId),
    CollapseBone(BoneId),
    SetEntityVisible(Entity, Visibility),
    SetEntityTranslation(Entity, Vector3<f32>),
    SetEntityRotation(Entity, Quaternion<f32>),
    SetEntityScale(Entity, Vector3<f32>),
    RenameEntity(Entity, String),
    NavigateTree {
        navigation: TreeNavigation,
        page_rows: usize,
    },
    ScrollToSelectionDone,
    SetBoneDisplayStyle(BoneDisplayStyle),
    SetBoneInFront(bool),
    SetBoneDistanceScaling(bool),
    SetBoneDistanceScaleFactor(f32),
}

impl UiCommand for HierarchyEvent {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        dispatch_hierarchy_events(&[*self], world, assets);
    }
}

fn dispatch_hierarchy_events(events: &[HierarchyEvent], world: &mut World, assets: &AssetStorage) {
    dispatch_hierarchy_entity_events(events, world);
    dispatch_hierarchy_bone_events(events, world, assets);
    sync_curve_editor_on_selection(events, world, assets);
    dispatch_hierarchy_navigation_events(events, world, assets);
}

fn dispatch_hierarchy_navigation_events(
    events: &[HierarchyEvent],
    world: &mut World,
    assets: &AssetStorage,
) {
    for event in events {
        match event {
            HierarchyEvent::NavigateTree {
                navigation,
                page_rows,
            } => match navigate_hierarchy(world, assets, *navigation, *page_rows) {
                TreeSelection::Entity(entity) => sync_curve_editor_on_selection(
                    &[HierarchyEvent::SelectEntity(entity)],
                    world,
                    assets,
                ),
                TreeSelection::Bone(bone_id) => {
                    dispatch_hierarchy_bone_events(
                        &[HierarchyEvent::SelectBone(bone_id)],
                        world,
                        assets,
                    );
                    sync_curve_editor_on_selection(
                        &[HierarchyEvent::SelectBone(bone_id)],
                        world,
                        assets,
                    );
                }
                TreeSelection::Unchanged => {}
            },
            HierarchyEvent::ScrollToSelectionDone => {
                world.resource_mut::<HierarchyState>().scroll_to_selected = false;
            }
            _ => {}
        }
    }
}

fn dispatch_hierarchy_entity_events(events: &[HierarchyEvent], world: &mut World) {
    for event in events {
        match event {
            HierarchyEvent::SelectEntity(entity) => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_select(&mut hierarchy_state, *entity);
            }

            HierarchyEvent::DeselectAll => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_deselect_all(&mut hierarchy_state);
            }

            HierarchyEvent::ToggleEntitySelection(entity) => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_toggle_selection(&mut hierarchy_state, *entity);
            }

            HierarchyEvent::ExpandEntity(entity) => {
                expand_entity(world, *entity);
            }

            HierarchyEvent::CollapseEntity(entity) => {
                collapse_entity(world, *entity);
            }

            HierarchyEvent::SetSearchFilter(filter) => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_state.search_filter = filter.clone();
            }

            HierarchyEvent::SetHierarchyDisplayMode(mode) => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_state.display_mode = *mode;
            }

            HierarchyEvent::SetEntityVisible(entity, visible) => {
                update_entity_visible(world, *entity, *visible);
            }

            HierarchyEvent::SetEntityTranslation(entity, translation) => {
                update_entity_translation(world, *entity, *translation);
            }

            HierarchyEvent::SetEntityRotation(entity, rotation) => {
                let target = resolve_transform_entity(world, *entity);
                if let Some(transform) = world.get_component_mut::<Transform>(target) {
                    transform.rotation = *rotation;
                }
            }

            HierarchyEvent::SetEntityScale(entity, scale) => {
                update_entity_scale(world, *entity, *scale);
            }

            HierarchyEvent::RenameEntity(entity, new_name) => {
                rename_entity(world, *entity, new_name.clone());
            }

            HierarchyEvent::DeleteSelectedEntities => {
                let hierarchy_state = world.resource::<HierarchyState>();
                let selected = hierarchy_state.selected_entity;
                let multi = hierarchy_state.multi_selection.clone();
                drop(hierarchy_state);

                let mut targets: Vec<Entity> = multi.into_iter().collect();
                if let Some(entity) = selected {
                    if !targets.contains(&entity) {
                        targets.push(entity);
                    }
                }

                let mut all_to_delete = Vec::new();
                for entity in &targets {
                    collect_entity_tree(world, *entity, &mut all_to_delete);
                }

                if !all_to_delete.is_empty() {
                    let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                    hierarchy_state.selected_entity = None;
                    hierarchy_state.multi_selection.clear();

                    world.resource_mut::<EntityRemovalQueue>().push(
                        EntityRemovalCommand::DeleteEntities {
                            entities: all_to_delete,
                        },
                    );
                }
            }

            _ => {}
        }
    }
}

fn dispatch_hierarchy_bone_events(
    events: &[HierarchyEvent],
    world: &mut World,
    assets: &AssetStorage,
) {
    for event in events {
        match event {
            HierarchyEvent::SelectBone(bone_id) => {
                let descendants: Vec<usize> = assets
                    .skeletons
                    .values()
                    .next()
                    .map(|skel_asset| {
                        skel_asset
                            .skeleton
                            .collect_descendants(*bone_id)
                            .into_iter()
                            .map(|id| id as usize)
                            .collect()
                    })
                    .unwrap_or_default();

                {
                    let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                    hierarchy_select_bone(&mut hierarchy_state, *bone_id);
                }

                if let Some(mut selection) = world.get_resource_mut::<BoneSelectionState>() {
                    let bone_idx = *bone_id as usize;
                    selection.selected_bone_indices.clear();
                    selection.selected_bone_indices.insert(bone_idx);
                    for desc_idx in descendants {
                        selection.selected_bone_indices.insert(desc_idx);
                    }
                    selection.active_bone_index = Some(bone_idx);
                }
            }

            HierarchyEvent::DeselectBone => {
                {
                    let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                    hierarchy_deselect_bone(&mut hierarchy_state);
                }

                if let Some(mut selection) = world.get_resource_mut::<BoneSelectionState>() {
                    selection.selected_bone_indices.clear();
                    selection.active_bone_index = None;
                }
            }

            HierarchyEvent::ExpandBone(bone_id) => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_expand_bone(&mut hierarchy_state, *bone_id);
            }

            HierarchyEvent::CollapseBone(bone_id) => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_collapse_bone(&mut hierarchy_state, *bone_id);
            }

            HierarchyEvent::SetBoneDisplayStyle(style) => {
                if let Some(mut bone_gizmo) = world.get_resource_mut::<BoneGizmoData>() {
                    bone_gizmo.display_style = *style;
                }
            }

            HierarchyEvent::SetBoneInFront(in_front) => {
                if let Some(mut bone_gizmo) = world.get_resource_mut::<BoneGizmoData>() {
                    bone_gizmo.in_front = *in_front;
                }
            }

            HierarchyEvent::SetBoneDistanceScaling(enabled) => {
                if let Some(mut bone_gizmo) = world.get_resource_mut::<BoneGizmoData>() {
                    bone_gizmo.distance_scaling_enabled = *enabled;
                }
            }

            HierarchyEvent::SetBoneDistanceScaleFactor(factor) => {
                if let Some(mut bone_gizmo) = world.get_resource_mut::<BoneGizmoData>() {
                    bone_gizmo.distance_scaling_factor = *factor;
                }
            }

            _ => {}
        }
    }
}

fn sync_curve_editor_on_selection(
    events: &[HierarchyEvent],
    world: &mut World,
    assets: &AssetStorage,
) {
    let is_open = world
        .get_resource::<CurveEditorState>()
        .map(|s| s.is_open)
        .unwrap_or(false);
    if !is_open {
        return;
    }

    for event in events {
        match event {
            HierarchyEvent::SelectEntity(entity) => {
                let clip_library = world.resource::<ClipLibrary>();
                let source_id = world.resource::<TimelineState>().current_clip_id;
                let bone_id =
                    resolve_mesh_bone_id(world, *entity, assets, &clip_library, source_id);
                drop(clip_library);

                if let Some(bone_id) = bone_id {
                    let mut editor = world.resource_mut::<CurveEditorState>();
                    editor.select_bone(bone_id);
                }
            }

            HierarchyEvent::SelectBone(bone_id) => {
                let has_track = {
                    let clip_library = world.resource::<ClipLibrary>();
                    let source_id = world.resource::<TimelineState>().current_clip_id;
                    source_id
                        .and_then(|id| clip_library.get(id))
                        .map(|clip| clip.tracks.contains_key(bone_id))
                        .unwrap_or(false)
                };

                if has_track {
                    let mut editor = world.resource_mut::<CurveEditorState>();
                    editor.select_bone(*bone_id);
                }
            }

            _ => {}
        }
    }
}

fn collect_entity_tree(world: &World, entity: Entity, out: &mut Vec<Entity>) {
    if out.contains(&entity) {
        return;
    }
    out.push(entity);

    let children = world
        .get_component::<Children>(entity)
        .map(|c| c.0.clone())
        .unwrap_or_default();

    for child in children {
        collect_entity_tree(world, child, out);
    }
}
