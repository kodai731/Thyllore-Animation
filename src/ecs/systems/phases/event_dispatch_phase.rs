use cgmath::Vector3;

use crate::asset::AssetStorage;
use crate::debugview::gizmo::{BoneGizmoData, BoneSelectionState};
use crate::debugview::RayTracingDebugState;
use crate::ecs::component::ClipSchedule;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{
    BonePoseOverride, Camera, ClipLibrary, CurveEditorBuffer, EditHistory, HierarchyState,
    KeyframeCopyBuffer, TimelineState,
};
use crate::ecs::systems::{
    apply_redo, apply_undo, camera_move_to_look_at, camera_reset, collapse_entity, expand_entity,
    hierarchy_collapse_bone, hierarchy_deselect_all, hierarchy_deselect_bone,
    hierarchy_expand_bone, hierarchy_select, hierarchy_select_bone, hierarchy_toggle_selection,
    process_bone_set_key, process_clip_instance_events, process_keyframe_clipboard_events,
    rename_entity, resolve_mesh_bone_id, timeline_process_events, update_entity_scale,
    update_entity_translation, update_entity_visible,
};
use crate::ecs::world::{Entity, Transform, World};
use crate::ecs::UIEventQueue;
use crate::platform::ui::CurveEditorState;

use super::super::ui_event_systems::DeferredAction;

pub fn run_event_dispatch_phase(
    world: &mut World,
    assets: &mut AssetStorage,
    model_bounds: Option<(Vector3<f32>, Vector3<f32>, Vector3<f32>)>,
) -> (Vec<UIEvent>, Vec<DeferredAction>) {
    let events: Vec<UIEvent> = {
        if let Some(mut ui_events) = world.get_resource_mut::<UIEventQueue>() {
            ui_events.drain().collect()
        } else {
            return (Vec::new(), Vec::new());
        }
    };

    if events.is_empty() {
        return (Vec::new(), Vec::new());
    }

    dispatch_hierarchy_events(&events, world, assets);
    dispatch_timeline_events(&events, world, assets);
    dispatch_keyframe_clipboard_events(&events, world);
    dispatch_buffer_events(&events, world);
    dispatch_clip_instance_events(&events, world);
    dispatch_clip_browser_ecs_events(&events, world, assets);
    dispatch_edit_history_events(&events, world);
    dispatch_scene_events(&events, world);
    dispatch_debug_constraint_events(&events, world, assets);
    dispatch_constraint_edit_events(&events, world);
    dispatch_constraint_bake_events(&events, world, assets);
    dispatch_spring_bone_bake_ecs_events(&events, world, assets);
    dispatch_spring_bone_edit_events(&events, world, assets);
    #[cfg(feature = "ml")]
    dispatch_curve_suggestion_events(&events, world);
    #[cfg(feature = "text-to-motion")]
    dispatch_text_to_motion_events(&events, world, assets);

    let deferred = dispatch_camera_light_debug_events(&events, world, model_bounds);
    let platform_events = filter_platform_events(&events);

    (platform_events, deferred)
}

fn filter_platform_events(events: &[UIEvent]) -> Vec<UIEvent> {
    events
        .iter()
        .filter(|e| {
            matches!(
                e,
                UIEvent::ClipBrowserLoadFromFile
                    | UIEvent::ClipBrowserSaveToFile(_)
                    | UIEvent::ClipBrowserExportFbx(_)
                    | UIEvent::SpringBoneSaveBake
            )
        })
        .cloned()
        .collect()
}

fn dispatch_camera_light_debug_events(
    events: &[UIEvent],
    world: &mut World,
    model_bounds: Option<(Vector3<f32>, Vector3<f32>, Vector3<f32>)>,
) -> Vec<DeferredAction> {
    let mut camera = world.resource_mut::<Camera>();
    let mut rt_debug = world.resource_mut::<RayTracingDebugState>();
    let mut deferred = Vec::new();

    for event in events {
        match event {
            UIEvent::ResetCamera | UIEvent::ResetCameraUp => {
                camera_reset(&mut camera);
            }

            UIEvent::MoveCameraToModel => {
                if let Some((min, max, center)) = model_bounds {
                    let size = max - min;
                    let max_dim = size.x.max(size.y).max(size.z);
                    let distance = max_dim * 2.0;
                    let offset = Vector3::new(0.0, 0.0, distance);
                    camera_move_to_look_at(&mut camera, center, offset);
                    crate::log!(
                        "Moved camera to model: center=({:.2}, {:.2}, {:.2}), distance={:.2}",
                        center.x,
                        center.y,
                        center.z,
                        distance
                    );
                }
            }

            UIEvent::MoveCameraToLightGizmo => {
                let light_pos = rt_debug.light_position;
                let offset = Vector3::new(2.0, 2.0, 2.0);
                camera_move_to_look_at(&mut camera, light_pos, offset);
            }

            UIEvent::SetLightPosition(pos) => {
                rt_debug.light_position = *pos;
            }

            UIEvent::MoveLightToBounds(target) => {
                use crate::app::data::LightMoveTarget;

                if let Some((min, max, _)) = model_bounds {
                    let offset = 2.0;
                    let current = rt_debug.light_position;
                    let new_pos = match target {
                        LightMoveTarget::XMin => Vector3::new(min.x - offset, current.y, current.z),
                        LightMoveTarget::XMax => Vector3::new(max.x + offset, current.y, current.z),
                        LightMoveTarget::YMin => Vector3::new(current.x, min.y - offset, current.z),
                        LightMoveTarget::YMax => Vector3::new(current.x, max.y + offset, current.z),
                        LightMoveTarget::ZMin => Vector3::new(current.x, current.y, min.z - offset),
                        LightMoveTarget::ZMax => Vector3::new(current.x, current.y, max.z + offset),
                        LightMoveTarget::None => current,
                    };
                    rt_debug.light_position = new_pos;

                    crate::log!(
                        "Light moved to bounds {:?}: ({:.2}, {:.2}, {:.2})",
                        target,
                        new_pos.x,
                        new_pos.y,
                        new_pos.z
                    );
                }
            }

            UIEvent::LoadModel { path } => {
                deferred.push(DeferredAction::LoadModel { path: path.clone() });
            }

            UIEvent::TakeScreenshot => {
                deferred.push(DeferredAction::TakeScreenshot);
            }

            UIEvent::DebugShadowInfo => {
                deferred.push(DeferredAction::DebugShadowInfo);
            }

            UIEvent::DebugBillboardDepth => {
                deferred.push(DeferredAction::DebugBillboardDepth);
            }

            UIEvent::DumpDebugInfo => {
                deferred.push(DeferredAction::DumpDebugInfo);
            }

            UIEvent::DumpAnimationDebug => {
                deferred.push(DeferredAction::DumpAnimationDebug);
            }

            _ => {}
        }
    }

    deferred
}

fn dispatch_hierarchy_events(events: &[UIEvent], world: &mut World, assets: &AssetStorage) {
    dispatch_hierarchy_entity_events(events, world);
    dispatch_hierarchy_bone_events(events, world, assets);
    sync_curve_editor_on_selection(events, world, assets);
}

fn dispatch_hierarchy_entity_events(events: &[UIEvent], world: &mut World) {
    for event in events {
        match event {
            UIEvent::SelectEntity(entity) => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_select(&mut hierarchy_state, *entity);
            }

            UIEvent::DeselectAll => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_deselect_all(&mut hierarchy_state);
            }

            UIEvent::ToggleEntitySelection(entity) => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_toggle_selection(&mut hierarchy_state, *entity);
            }

            UIEvent::ExpandEntity(entity) => {
                expand_entity(world, *entity);
            }

            UIEvent::CollapseEntity(entity) => {
                collapse_entity(world, *entity);
            }

            UIEvent::SetSearchFilter(filter) => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_state.search_filter = filter.clone();
            }

            UIEvent::SetHierarchyDisplayMode(mode) => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_state.display_mode = *mode;
            }

            UIEvent::SetEntityVisible(entity, visible) => {
                update_entity_visible(world, *entity, *visible);
            }

            UIEvent::SetEntityTranslation(entity, translation) => {
                update_entity_translation(world, *entity, *translation);
            }

            UIEvent::SetEntityRotation(entity, rotation) => {
                if let Some(transform) = world.get_component_mut::<Transform>(*entity) {
                    transform.rotation = *rotation;
                }
            }

            UIEvent::SetEntityScale(entity, scale) => {
                update_entity_scale(world, *entity, *scale);
            }

            UIEvent::RenameEntity(entity, new_name) => {
                rename_entity(world, *entity, new_name.clone());
            }

            UIEvent::FocusOnEntity(entity) => {
                let target = world
                    .get_component::<Transform>(*entity)
                    .map(|t| t.translation);

                if let Some(target) = target {
                    let offset = Vector3::new(5.0, 3.0, 5.0);
                    let mut camera = world.resource_mut::<Camera>();
                    camera_move_to_look_at(&mut camera, target, offset);
                }
            }

            _ => {}
        }
    }
}

fn dispatch_hierarchy_bone_events(events: &[UIEvent], world: &mut World, assets: &AssetStorage) {
    for event in events {
        match event {
            UIEvent::SelectBone(bone_id) => {
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

            UIEvent::DeselectBone => {
                {
                    let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                    hierarchy_deselect_bone(&mut hierarchy_state);
                }

                if let Some(mut selection) = world.get_resource_mut::<BoneSelectionState>() {
                    selection.selected_bone_indices.clear();
                    selection.active_bone_index = None;
                }
            }

            UIEvent::ExpandBone(bone_id) => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_expand_bone(&mut hierarchy_state, *bone_id);
            }

            UIEvent::CollapseBone(bone_id) => {
                let mut hierarchy_state = world.resource_mut::<HierarchyState>();
                hierarchy_collapse_bone(&mut hierarchy_state, *bone_id);
            }

            UIEvent::SetBoneDisplayStyle(style) => {
                if let Some(mut bone_gizmo) = world.get_resource_mut::<BoneGizmoData>() {
                    bone_gizmo.display_style = *style;
                }
            }

            UIEvent::SetBoneInFront(in_front) => {
                if let Some(mut bone_gizmo) = world.get_resource_mut::<BoneGizmoData>() {
                    bone_gizmo.in_front = *in_front;
                }
            }

            UIEvent::SetBoneDistanceScaling(enabled) => {
                if let Some(mut bone_gizmo) = world.get_resource_mut::<BoneGizmoData>() {
                    bone_gizmo.distance_scaling_enabled = *enabled;
                }
            }

            UIEvent::SetBoneDistanceScaleFactor(factor) => {
                if let Some(mut bone_gizmo) = world.get_resource_mut::<BoneGizmoData>() {
                    bone_gizmo.distance_scaling_factor = *factor;
                }
            }

            _ => {}
        }
    }
}

fn sync_curve_editor_on_selection(events: &[UIEvent], world: &mut World, assets: &AssetStorage) {
    let is_open = world
        .get_resource::<CurveEditorState>()
        .map(|s| s.is_open)
        .unwrap_or(false);
    if !is_open {
        return;
    }

    for event in events {
        match event {
            UIEvent::SelectEntity(entity) => {
                let clip_library = world.resource::<ClipLibrary>();
                let source_id = world.resource::<TimelineState>().current_clip_id;
                let bone_id =
                    resolve_mesh_bone_id(world, *entity, assets, &clip_library, source_id);
                drop(clip_library);

                if let Some(bone_id) = bone_id {
                    let mut editor = world.resource_mut::<CurveEditorState>();
                    editor.selected_bone_id = Some(bone_id);
                }
            }

            UIEvent::SelectBone(bone_id) => {
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
                    editor.selected_bone_id = Some(*bone_id);
                }
            }

            _ => {}
        }
    }
}

fn dispatch_timeline_events(events: &[UIEvent], world: &mut World, assets: &AssetStorage) {
    let mut timeline_state = world.resource_mut::<TimelineState>();
    let mut clip_library = world.resource_mut::<ClipLibrary>();

    let clip_id = timeline_state.current_clip_id;
    let before_clip = clip_id.and_then(|id| clip_library.get(id).cloned());

    let modified = timeline_process_events(events, &mut timeline_state, &mut *clip_library);

    if modified {
        if let (Some(cid), Some(before)) = (clip_id, before_clip) {
            if let Some(after) = clip_library.get(cid).cloned() {
                if world.contains_resource::<EditHistory>() {
                    let mut edit_history = world.resource_mut::<EditHistory>();
                    edit_history.push_clip_edit(cid, before, after, "timeline clip edit");
                }
            }
        }
    }

    drop(clip_library);
    drop(timeline_state);

    if modified {
        transition_to_baked_override_if_needed(world);
    }

    for event in events {
        match event {
            UIEvent::TimelineSelectClip(source_id) => {
                let lib = world.resource::<ClipLibrary>();
                let duration = lib.get(*source_id).map(|c| c.duration).unwrap_or(1.0);
                let asset_id = lib.get_asset_id_for_source(*source_id);
                crate::log!(
                    "[ClipSelect] source_id={}, asset_id={:?}, duration={:.3}",
                    source_id,
                    asset_id,
                    duration,
                );
                drop(lib);

                let schedule_entities = world.component_entities::<ClipSchedule>();
                for entity in &schedule_entities {
                    if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                        if let Some(first) = schedule.instances.first_mut() {
                            first.source_id = *source_id;
                            first.clip_out = duration;
                        }
                    }
                }
            }

            UIEvent::TimelinePlay => {
                if let Some(mut overrides) = world.get_resource_mut::<BonePoseOverride>() {
                    overrides.clear();
                }
            }

            _ => {}
        }
    }

    dispatch_bone_set_key_events(events, world, assets);
}

fn dispatch_bone_set_key_events(events: &[UIEvent], world: &mut World, assets: &AssetStorage) {
    if !events.iter().any(|e| matches!(e, UIEvent::BoneSetKey)) {
        return;
    }

    let overrides = match world.get_resource::<BonePoseOverride>() {
        Some(r) => r.overrides.clone(),
        None => return,
    };
    if overrides.is_empty() {
        return;
    }

    let skeleton_id = world
        .get_resource::<crate::debugview::gizmo::BoneGizmoData>()
        .and_then(|bg| bg.cached_skeleton_id);
    let Some(skel_id) = skeleton_id else { return };
    let Some(skeleton) = assets.get_skeleton_by_skeleton_id(skel_id) else {
        return;
    };
    let skeleton = skeleton.clone();

    let timeline_state = world.resource::<TimelineState>();
    let mut clip_library = world.resource_mut::<ClipLibrary>();

    let clip_id = timeline_state.current_clip_id;
    let before_clip = clip_id.and_then(|id| clip_library.get(id).cloned());

    let modified = process_bone_set_key(&overrides, &mut clip_library, &timeline_state, &skeleton);

    if modified {
        if let (Some(cid), Some(before)) = (clip_id, before_clip) {
            if let Some(after) = clip_library.get(cid).cloned() {
                if world.contains_resource::<EditHistory>() {
                    let mut edit_history = world.resource_mut::<EditHistory>();
                    edit_history.push_clip_edit(cid, before, after, "bone set key");
                }
            }
        }

        if let Some(mut pose_overrides) = world.get_resource_mut::<BonePoseOverride>() {
            pose_overrides.clear();
        }
    }
}

fn dispatch_keyframe_clipboard_events(events: &[UIEvent], world: &mut World) {
    let timeline_state = world.resource::<TimelineState>();
    let mut clip_library = world.resource_mut::<ClipLibrary>();
    let mut copy_buffer = world.resource_mut::<KeyframeCopyBuffer>();

    process_keyframe_clipboard_events(
        events,
        &*timeline_state,
        &mut *clip_library,
        &mut *copy_buffer,
    );
}

fn dispatch_buffer_events(events: &[UIEvent], world: &mut World) {
    for event in events {
        match event {
            UIEvent::TimelineCaptureBuffer => {
                let timeline_state = world.resource::<TimelineState>();
                let clip_library = world.resource::<ClipLibrary>();
                let curve_editor = world.resource::<CurveEditorState>();
                let mut curve_buffer = world.resource_mut::<CurveEditorBuffer>();

                if let Some(bone_id) = curve_editor.selected_bone_id {
                    if let Some(clip_id) = timeline_state.current_clip_id {
                        if let Some(clip) = clip_library.get(clip_id) {
                            crate::ecs::systems::curve_editor_capture_buffer(
                                &mut curve_buffer,
                                clip,
                                bone_id,
                                &curve_editor.visible_curves,
                                clip.duration,
                                100,
                            );
                        }
                    }
                }
            }

            UIEvent::TimelineSwapBuffer => {
                let curve_editor = world.resource::<CurveEditorState>();
                let timeline_state = world.resource::<TimelineState>();
                let mut clip_library = world.resource_mut::<ClipLibrary>();
                let mut curve_buffer = world.resource_mut::<CurveEditorBuffer>();

                if let Some(bone_id) = curve_editor.selected_bone_id {
                    if let Some(clip_id) = timeline_state.current_clip_id {
                        if let Some(clip) = clip_library.get_mut(clip_id) {
                            crate::ecs::systems::curve_editor_swap_buffer(
                                &mut curve_buffer,
                                clip,
                                bone_id,
                            );
                        }
                    }
                }
            }

            _ => {}
        }
    }
}

fn dispatch_clip_instance_events(events: &[UIEvent], world: &mut World) {
    let schedule_snapshots = collect_clip_schedule_snapshots(events, world);

    process_clip_instance_events(events, world);

    for event in events {
        if let UIEvent::ClipInstanceSelect {
            entity,
            instance_id,
        } = event
        {
            let _source_id = world
                .get_component::<ClipSchedule>(*entity)
                .and_then(|schedule| {
                    schedule
                        .instances
                        .iter()
                        .find(|i| i.instance_id == *instance_id)
                        .map(|i| i.source_id)
                });
        }
    }

    record_schedule_changes(schedule_snapshots, world);
}

fn collect_clip_schedule_snapshots(
    events: &[UIEvent],
    world: &World,
) -> Vec<(Entity, ClipSchedule)> {
    use std::collections::HashSet;

    let mut entities = HashSet::new();
    for event in events {
        match event {
            UIEvent::ClipInstanceMove { entity, .. }
            | UIEvent::ClipInstanceTrimStart { entity, .. }
            | UIEvent::ClipInstanceTrimEnd { entity, .. }
            | UIEvent::ClipInstanceToggleMute { entity, .. }
            | UIEvent::ClipInstanceDelete { entity, .. }
            | UIEvent::ClipInstanceSetWeight { entity, .. }
            | UIEvent::ClipInstanceSetBlendMode { entity, .. }
            | UIEvent::ClipGroupCreate { entity, .. }
            | UIEvent::ClipGroupDelete { entity, .. }
            | UIEvent::ClipGroupAddInstance { entity, .. }
            | UIEvent::ClipGroupRemoveInstance { entity, .. }
            | UIEvent::ClipGroupToggleMute { entity, .. }
            | UIEvent::ClipGroupSetWeight { entity, .. } => {
                entities.insert(*entity);
            }
            _ => {}
        }
    }

    entities
        .into_iter()
        .filter_map(|entity| {
            world
                .get_component::<ClipSchedule>(entity)
                .cloned()
                .map(|s| (entity, s))
        })
        .collect()
}

fn record_schedule_changes(snapshots: Vec<(Entity, ClipSchedule)>, world: &mut World) {
    if snapshots.is_empty() {
        return;
    }

    if !world.contains_resource::<EditHistory>() {
        return;
    }

    for (entity, before) in snapshots {
        let after = world.get_component::<ClipSchedule>(entity).cloned();

        if let Some(after) = after {
            let changed = before.instances.len() != after.instances.len()
                || before.groups.len() != after.groups.len()
                || format!("{:?}", before) != format!("{:?}", after);

            if changed {
                let mut edit_history = world.resource_mut::<EditHistory>();
                edit_history.push_schedule_edit(entity, before, after, "clip schedule edit");
            }
        }
    }
}

fn dispatch_clip_browser_ecs_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &mut AssetStorage,
) {
    for event in events {
        match event {
            UIEvent::ClipInstanceAdd {
                entity,
                source_id,
                start_time,
            } => {
                let duration = {
                    let clip_library = world.resource::<ClipLibrary>();
                    clip_library
                        .get(*source_id)
                        .map(|c| c.duration)
                        .unwrap_or(1.0)
                };

                if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                    crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance(
                        schedule, *source_id, duration,
                    );

                    if let Some(last) = schedule.instances.last_mut() {
                        last.start_time = *start_time;
                    }
                }
            }

            UIEvent::ClipBrowserCreateEmpty => {
                let mut clip_library = world.resource_mut::<ClipLibrary>();
                let editable = crate::animation::editable::EditableAnimationClip::new(
                    0,
                    "New Clip".to_string(),
                );
                let id =
                    crate::ecs::systems::clip_library_systems::clip_library_register_and_activate(
                        &mut clip_library,
                        assets,
                        editable,
                    );
                crate::log!("Created empty clip (id={})", id);
            }

            UIEvent::ClipBrowserDuplicate(source_id) => {
                let mut clip_library = world.resource_mut::<ClipLibrary>();
                if let Some(original) = clip_library.get(*source_id).cloned() {
                    let mut duplicate = original;
                    duplicate.name = format!("{} (copy)", duplicate.name);
                    let new_id =
                        crate::ecs::systems::clip_library_systems::clip_library_register_and_activate(
                            &mut clip_library,
                            assets,
                            duplicate,
                        );
                    crate::log!("Duplicated clip {} -> {}", source_id, new_id);
                }
            }

            UIEvent::ClipBrowserDelete(source_id) => {
                let ref_count = count_source_references(*source_id, world);
                if ref_count == 0 {
                    let mut clip_library = world.resource_mut::<ClipLibrary>();
                    clip_library.remove(*source_id);
                    crate::log!("Deleted clip (id={})", source_id);
                } else {
                    crate::log!(
                        "Cannot delete clip {}: {} references remain",
                        source_id,
                        ref_count
                    );
                }
            }

            _ => {}
        }
    }
}

fn count_source_references(
    source_id: crate::animation::editable::SourceClipId,
    world: &World,
) -> usize {
    let entities = world.component_entities::<ClipSchedule>();
    let mut count = 0;
    for entity in entities {
        if let Some(schedule) = world.get_component::<ClipSchedule>(entity) {
            count += schedule
                .instances
                .iter()
                .filter(|i| i.source_id == source_id)
                .count();
        }
    }
    count
}

fn dispatch_edit_history_events(events: &[UIEvent], world: &mut World) {
    for event in events {
        match event {
            UIEvent::Undo => {
                if !world.contains_resource::<EditHistory>() {
                    return;
                }

                let mut edit_history = world.resource_mut::<EditHistory>();
                if !edit_history.can_undo() {
                    return;
                }
                let edit_history_ptr: *mut EditHistory = &mut *edit_history;
                drop(edit_history);

                let mut clip_library = world.resource_mut::<ClipLibrary>();
                let clip_library_ptr: *mut ClipLibrary = &mut *clip_library;
                drop(clip_library);

                unsafe {
                    apply_undo(&mut *edit_history_ptr, &mut *clip_library_ptr, world);
                }
            }

            UIEvent::Redo => {
                if !world.contains_resource::<EditHistory>() {
                    return;
                }

                let mut edit_history = world.resource_mut::<EditHistory>();
                if !edit_history.can_redo() {
                    return;
                }
                let edit_history_ptr: *mut EditHistory = &mut *edit_history;
                drop(edit_history);

                let mut clip_library = world.resource_mut::<ClipLibrary>();
                let clip_library_ptr: *mut ClipLibrary = &mut *clip_library;
                drop(clip_library);

                unsafe {
                    apply_redo(&mut *edit_history_ptr, &mut *clip_library_ptr, world);
                }
            }

            _ => {}
        }
    }
}

fn dispatch_scene_events(events: &[UIEvent], world: &mut World) {
    for event in events {
        if let UIEvent::SaveScene = event {
            let scene_path = std::path::PathBuf::from("assets/scenes/default.scene.ron");

            match crate::scene::save_scene(&scene_path, world) {
                Ok(()) => {
                    crate::log!("Scene saved to {:?}", scene_path);
                }
                Err(e) => {
                    crate::log!("Failed to save scene: {:?}", e);
                }
            }
        }
    }
}

fn dispatch_debug_constraint_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &mut AssetStorage,
) {
    use crate::ecs::systems::debug_constraint_systems::{
        clear_test_constraints, create_test_constraints,
    };
    use crate::ecs::systems::debug_spring_bone_systems::{
        clear_spring_bones, create_test_spring_bones,
    };

    for event in events {
        match event {
            UIEvent::CreateTestConstraints => {
                create_test_constraints(world, assets);
            }
            UIEvent::ClearTestConstraints => {
                clear_test_constraints(world);
            }
            UIEvent::AddTestSpringBones => {
                create_test_spring_bones(world, assets);
            }
            UIEvent::ClearSpringBones => {
                let is_baked = world
                    .get_resource::<crate::ecs::resource::SpringBoneState>()
                    .map_or(false, |s| s.baked_clip_source_id.is_some());
                if is_baked {
                    handle_spring_bone_discard(world, assets);
                }
                clear_spring_bones(world);
            }
            _ => {}
        }
    }
}

fn dispatch_constraint_edit_events(events: &[UIEvent], world: &mut World) {
    use crate::ecs::systems::constraint_edit_systems::{
        handle_constraint_add, handle_constraint_remove, handle_constraint_update,
    };

    for event in events {
        match event {
            UIEvent::ConstraintAdd {
                entity,
                constraint_type_index,
            } => {
                handle_constraint_add(world, *entity, *constraint_type_index);
            }
            UIEvent::ConstraintRemove {
                entity,
                constraint_id,
            } => {
                handle_constraint_remove(world, *entity, *constraint_id);
            }
            UIEvent::ConstraintUpdate {
                entity,
                constraint_id,
                constraint,
            } => {
                handle_constraint_update(world, *entity, *constraint_id, constraint);
            }
            _ => {}
        }
    }
}

fn dispatch_constraint_bake_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &mut AssetStorage,
) {
    use crate::ecs::component::ConstraintSet;
    use crate::ecs::resource::{ClipLibrary, TimelineState};
    use crate::ecs::systems::constraint_bake_systems::{
        constraint_bake_evaluate, constraint_bake_register, constraint_bake_rest_pose,
    };

    for event in events {
        let UIEvent::ConstraintBakeToKeyframes { entity, sample_fps } = event else {
            continue;
        };

        let skeleton = match assets.skeletons.values().next() {
            Some(skel_asset) => skel_asset.skeleton.clone(),
            None => {
                crate::log!("Bake failed: no skeleton found");
                continue;
            }
        };

        let constraint_set = match world.get_component::<ConstraintSet>(*entity) {
            Some(set) => set.clone(),
            None => {
                crate::log!("Bake failed: no ConstraintSet on entity");
                continue;
            }
        };

        let timeline_state = world.resource::<TimelineState>();
        let clip_id = timeline_state.current_clip_id;
        let looping = timeline_state.looping;
        drop(timeline_state);

        let mut baked = if let Some(source_id) = clip_id {
            let clip_library = world.resource::<ClipLibrary>();
            match clip_library.get(source_id) {
                Some(editable) => {
                    let anim_clip = editable.to_animation_clip();
                    let source_name = editable.name.clone();
                    drop(clip_library);

                    let mut result = constraint_bake_evaluate(
                        &anim_clip,
                        &skeleton,
                        &constraint_set,
                        *sample_fps,
                        looping,
                    );
                    result.name = format!("{}_baked", source_name);
                    result
                }
                None => {
                    drop(clip_library);
                    constraint_bake_rest_pose(&skeleton, &constraint_set)
                }
            }
        } else {
            constraint_bake_rest_pose(&skeleton, &constraint_set)
        };

        baked.name = if baked.name.is_empty() {
            "baked".to_string()
        } else {
            baked.name
        };

        let mut clip_library = world.resource_mut::<ClipLibrary>();
        let new_id = constraint_bake_register(&mut clip_library, assets, baked);
        crate::log!("Baked constraints to new clip (id={})", new_id);
    }
}

fn dispatch_spring_bone_bake_ecs_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &mut AssetStorage,
) {
    for event in events {
        match event {
            UIEvent::SpringBoneBake => {
                handle_spring_bone_bake(world, assets);
            }
            UIEvent::SpringBoneDiscardBake => {
                handle_spring_bone_discard(world, assets);
            }
            UIEvent::SpringBoneRebake => {
                handle_spring_bone_discard(world, assets);
                handle_spring_bone_bake(world, assets);
            }
            _ => {}
        }
    }
}

fn transition_to_baked_override_if_needed(world: &mut World) {
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState};

    if let Some(mut state) = world.get_resource_mut::<SpringBoneState>() {
        if state.mode == SpringBoneMode::Baked {
            state.mode = SpringBoneMode::BakedOverride;
            crate::log!("Spring bone mode: Baked -> BakedOverride (manual edit detected)");
        }
    }
}

fn handle_spring_bone_bake(world: &mut World, assets: &mut AssetStorage) {
    use crate::ecs::component::{ConstraintSet, SpringBoneSetup, WithSpringBone};
    use crate::ecs::resource::{ClipLibrary, TimelineState};
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState};
    use crate::ecs::systems::spring_bone_bake_systems::{
        merge_bake_into_clip, spring_bone_bake, BakeConfig,
    };

    let skeleton = match assets.skeletons.values().next() {
        Some(skel_asset) => skel_asset.skeleton.clone(),
        None => {
            crate::log!("Spring bone bake failed: no skeleton found");
            return;
        }
    };

    let spring_entity = world
        .iter_components::<WithSpringBone>()
        .next()
        .map(|(entity, _)| entity);

    let Some(entity) = spring_entity else {
        crate::log!("Spring bone bake failed: no WithSpringBone entity");
        return;
    };

    let setup = match world.get_component::<SpringBoneSetup>(entity) {
        Some(s) => s.clone(),
        None => {
            crate::log!("Spring bone bake failed: no SpringBoneSetup");
            return;
        }
    };

    let constraints = world.get_component::<ConstraintSet>(entity).cloned();

    let timeline_state = world.resource::<TimelineState>();
    let source_id = timeline_state.current_clip_id;
    let looping = timeline_state.looping;
    drop(timeline_state);

    let clip_library = world.resource::<ClipLibrary>();
    let (anim_clip, source_editable) = match source_id.and_then(|id| clip_library.get(id)) {
        Some(editable) => (editable.to_animation_clip(), editable.clone()),
        None => {
            drop(clip_library);
            crate::log!("Spring bone bake failed: no current clip");
            return;
        }
    };
    drop(clip_library);

    let config = BakeConfig {
        start_time: 0.0,
        end_time: anim_clip.duration,
        sample_rate: 30.0,
    };

    let bake_result = spring_bone_bake(
        &config,
        &setup,
        &skeleton,
        &anim_clip,
        constraints.as_ref(),
        looping,
    );

    let mut merged = source_editable;
    merge_bake_into_clip(&mut merged, &bake_result, &skeleton);
    merged.name = format!("{}_spring_baked", merged.name);

    crate::log!(
        "[BakeDebug] bake_result: baked_bone_ids={:?}, clip_tracks={}",
        bake_result.baked_bone_ids,
        bake_result.clip.tracks.len()
    );
    crate::log!(
        "[BakeDebug] merged clip: name={}, tracks={}, duration={}",
        merged.name,
        merged.tracks.len(),
        merged.duration
    );

    let new_id = register_baked_clip_and_update_schedules(world, assets, merged, source_id);

    let baked_bone_ids = bake_result.baked_bone_ids.clone();

    let mut spring_state = world.resource_mut::<SpringBoneState>();
    spring_state.mode = SpringBoneMode::Baked;
    spring_state.baked_clip_source_id = Some(new_id);
    spring_state.baked_bone_ids = bake_result.baked_bone_ids;
    spring_state.original_clip_source_id = source_id;

    let mut timeline_state = world.resource_mut::<TimelineState>();
    timeline_state.current_clip_id = Some(new_id);
    timeline_state.baked_bone_ids = baked_bone_ids;

    crate::log!("Spring bone baked to new clip (id={})", new_id);
}

fn register_baked_clip_and_update_schedules(
    world: &mut World,
    assets: &mut AssetStorage,
    clip: crate::animation::editable::EditableAnimationClip,
    source_id: Option<u64>,
) -> u64 {
    use crate::ecs::systems::clip_library_systems::clip_library_register_and_activate;

    let mut clip_library = world.resource_mut::<ClipLibrary>();
    let new_id = clip_library_register_and_activate(&mut clip_library, assets, clip);
    drop(clip_library);

    let mut updated_count = 0;
    let schedule_entities = world.component_entities::<ClipSchedule>();
    crate::log!(
        "[BakeDebug] ClipSchedule entities count={}, original source_id={:?}",
        schedule_entities.len(),
        source_id
    );
    for sched_entity in &schedule_entities {
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*sched_entity) {
            if let Some(first) = schedule.instances.first_mut() {
                crate::log!(
                    "[BakeDebug]   entity {:?}: schedule source_id={}, match={}",
                    sched_entity,
                    first.source_id,
                    Some(first.source_id) == source_id
                );
                if Some(first.source_id) == source_id {
                    first.source_id = new_id;
                    updated_count += 1;
                }
            }
        }
    }
    crate::log!(
        "[BakeDebug] updated {} ClipSchedule(s) to new source_id={}",
        updated_count,
        new_id
    );

    new_id
}

fn handle_spring_bone_discard(world: &mut World, assets: &mut AssetStorage) {
    use crate::ecs::resource::ClipLibrary;
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState};

    let mut spring_state = world.resource_mut::<SpringBoneState>();
    let original_id = spring_state.original_clip_source_id;
    let baked_id = spring_state.baked_clip_source_id;

    spring_state.mode = SpringBoneMode::Realtime;
    spring_state.baked_clip_source_id = None;
    spring_state.baked_bone_ids = Vec::new();
    spring_state.original_clip_source_id = None;
    spring_state.initialized = false;
    drop(spring_state);

    if let (Some(orig_id), Some(baked_source_id)) = (original_id, baked_id) {
        let schedule_entities = world.component_entities::<ClipSchedule>();
        for entity in &schedule_entities {
            if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                if let Some(first) = schedule.instances.first_mut() {
                    if first.source_id == baked_source_id {
                        first.source_id = orig_id;
                    }
                }
            }
        }
    }

    if let Some(baked_id) = baked_id {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        if let Some(asset_id) = clip_library.source_to_asset_id.remove(&baked_id) {
            assets.animation_clips.remove(&asset_id);
        }
        clip_library.remove(baked_id);
    }

    let mut timeline_state = world.resource_mut::<TimelineState>();
    timeline_state.baked_bone_ids.clear();
    if let Some(orig) = original_id {
        timeline_state.current_clip_id = Some(orig);
    }

    crate::log!("Discarded spring bone bake, restored original clip");
}

fn dispatch_spring_bone_edit_events(events: &[UIEvent], world: &mut World, assets: &AssetStorage) {
    use crate::ecs::systems::spring_bone_edit_systems::*;

    let skeleton = assets
        .skeletons
        .values()
        .next()
        .map(|sa| sa.skeleton.clone());

    for event in events {
        match event {
            UIEvent::SpringChainAdd {
                entity,
                root_bone_id,
                chain_length,
            } => {
                if let Some(ref skel) = skeleton {
                    handle_spring_chain_add(world, *entity, *root_bone_id, *chain_length, skel);
                }
            }

            UIEvent::SpringChainRemove { entity, chain_id } => {
                handle_spring_chain_remove(world, *entity, *chain_id);
            }

            UIEvent::SpringChainUpdate {
                entity,
                chain_id,
                chain,
            } => {
                handle_spring_chain_update(world, *entity, *chain_id, chain.clone());
            }

            UIEvent::SpringJointUpdate {
                entity,
                chain_id,
                joint_index,
                joint,
            } => {
                handle_spring_joint_update(world, *entity, *chain_id, *joint_index, joint.clone());
            }

            UIEvent::SpringColliderAdd {
                entity,
                bone_id,
                shape,
            } => {
                handle_spring_collider_add(world, *entity, *bone_id, shape.clone());
            }

            UIEvent::SpringColliderRemove {
                entity,
                collider_id,
            } => {
                handle_spring_collider_remove(world, *entity, *collider_id);
            }

            UIEvent::SpringColliderUpdate {
                entity,
                collider_id,
                collider,
            } => {
                handle_spring_collider_update(world, *entity, *collider_id, collider.clone());
            }

            UIEvent::SpringColliderGroupAdd { entity, name } => {
                handle_spring_collider_group_add(world, *entity, name.clone());
            }

            UIEvent::SpringColliderGroupRemove { entity, group_id } => {
                handle_spring_collider_group_remove(world, *entity, *group_id);
            }

            UIEvent::SpringColliderGroupUpdate {
                entity,
                group_id,
                group,
            } => {
                handle_spring_collider_group_update(world, *entity, *group_id, group.clone());
            }

            UIEvent::SpringBoneToggleGizmo(visible) => {
                if let Some(mut gizmo) =
                    world.get_resource_mut::<crate::debugview::gizmo::SpringBoneGizmoData>()
                {
                    gizmo.visible = *visible;
                }
            }

            _ => {}
        }
    }
}

#[cfg(feature = "ml")]
fn dispatch_curve_suggestion_events(events: &[UIEvent], world: &mut World) {
    use crate::ecs::resource::{
        BoneNameTokenCache, BoneTopologyCache, CurveSuggestionState, InferenceActorState,
    };
    use crate::ecs::systems::{
        curve_suggestion_apply, curve_suggestion_dismiss, curve_suggestion_submit,
    };
    use crate::ml::CURVE_COPILOT_ACTOR_ID;

    for event in events {
        match event {
            UIEvent::CurveSuggestionRequest {
                bone_id,
                property_type,
            } => {
                let timeline_state = world.resource::<TimelineState>();
                let clip_id = timeline_state.current_clip_id;
                let current_time = timeline_state.current_time;
                drop(timeline_state);

                let clip_library = world.resource::<ClipLibrary>();
                let clip_info = clip_id
                    .and_then(|id| clip_library.get(id))
                    .and_then(|clip| {
                        clip.tracks
                            .get(bone_id)
                            .map(|track| (track.get_curve(*property_type).clone(), clip.duration))
                    });
                drop(clip_library);

                if let Some((curve, clip_duration)) = clip_info {
                    let topology_cache = world.resource::<BoneTopologyCache>();
                    let name_token_cache = world.resource::<BoneNameTokenCache>();
                    let mut suggestion_state = world.resource_mut::<CurveSuggestionState>();
                    let mut inference_state = world.resource_mut::<InferenceActorState>();
                    curve_suggestion_submit(
                        &mut suggestion_state,
                        &mut inference_state,
                        CURVE_COPILOT_ACTOR_ID,
                        &curve,
                        *property_type,
                        *bone_id,
                        clip_duration,
                        current_time,
                        &topology_cache,
                        &name_token_cache,
                    );
                }
            }

            UIEvent::CurveSuggestionAccept => {
                let suggestion = {
                    let state = world.resource::<CurveSuggestionState>();
                    state.suggestions.first().cloned()
                };

                if let Some(suggestion) = suggestion {
                    let timeline_state = world.resource::<TimelineState>();
                    let clip_id = timeline_state.current_clip_id;
                    drop(timeline_state);

                    if let Some(cid) = clip_id {
                        let mut clip_library = world.resource_mut::<ClipLibrary>();
                        if let Some(clip) = clip_library.get_mut(cid) {
                            if let Some(track) = clip.tracks.get_mut(&suggestion.bone_id) {
                                let curve = track.get_curve_mut(suggestion.property_type);
                                curve_suggestion_apply(&suggestion, curve);
                            }
                        }
                    }

                    let mut state = world.resource_mut::<CurveSuggestionState>();
                    curve_suggestion_dismiss(&mut state);
                    crate::log!("CurveCopilot: suggestion accepted");
                }
            }

            UIEvent::CurveSuggestionDismiss => {
                let mut state = world.resource_mut::<CurveSuggestionState>();
                curve_suggestion_dismiss(&mut state);
                crate::log!("CurveCopilot: suggestion dismissed");
            }

            _ => {}
        }
    }
}

#[cfg(feature = "text-to-motion")]
fn dispatch_text_to_motion_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &mut AssetStorage,
) {
    use crate::ecs::resource::TextToMotionState;
    use crate::ecs::systems::{text_to_motion_cancel, text_to_motion_submit};
    use crate::grpc::GrpcThreadHandle;

    const DEFAULT_ENDPOINT: &str = "http://localhost:50051";

    for event in events {
        match event {
            UIEvent::TextToMotionGenerate {
                prompt,
                duration_seconds,
            } => {
                if !world.contains_resource::<GrpcThreadHandle>() {
                    let handle = GrpcThreadHandle::spawn(DEFAULT_ENDPOINT);
                    world.insert_resource(handle);
                    crate::log!("TextToMotion: spawned gRPC thread ({})", DEFAULT_ENDPOINT);
                }

                let handle = world.get_resource::<GrpcThreadHandle>();
                let mut state = world.resource_mut::<TextToMotionState>();

                if let Some(handle) = handle {
                    text_to_motion_submit(&mut state, &*handle, prompt, *duration_seconds);
                }
            }

            UIEvent::TextToMotionApply => {
                let clip = {
                    let mut state = world.resource_mut::<TextToMotionState>();
                    state.generated_clip.take()
                };

                if let Some(clip) = clip {
                    let mut clip_library = world.resource_mut::<ClipLibrary>();
                    let new_id =
                        crate::ecs::systems::clip_library_systems::clip_library_register_and_activate(
                            &mut clip_library,
                            assets,
                            clip,
                        );
                    drop(clip_library);

                    let mut timeline = world.resource_mut::<TimelineState>();
                    timeline.current_clip_id = Some(new_id);

                    let mut state = world.resource_mut::<TextToMotionState>();
                    text_to_motion_cancel(&mut state);

                    crate::log!("TextToMotion: applied clip (id={})", new_id);
                }
            }

            UIEvent::TextToMotionCancel => {
                let mut state = world.resource_mut::<TextToMotionState>();
                text_to_motion_cancel(&mut state);
                crate::log!("TextToMotion: cancelled");
            }

            _ => {}
        }
    }
}
