use crate::asset::AssetStorage;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{ClipLibrary, CurveEditorState, EditHistory, TimelineState};
use crate::ecs::systems::scalar_clip_systems::{
    ensure_entity_clip, resolve_selected_scalar_entity, scalar_clip_clear_keys,
    scalar_clip_insert_debug_keys, scalar_clip_insert_key,
};
use crate::ecs::world::World;
use crate::hooks::effect_spawn::spawn_effect_instance;
use thyllore_anim_core::editable::SourceClipId;

/// Scalar keyframe events, applied to the resolved domain entity's clip. Undo
/// goes through the shared `ClipModified` path, so scalar edits merge and
/// revert exactly like bone clip edits.
pub fn dispatch_scalar_clip_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &mut AssetStorage,
) {
    for event in events {
        match event {
            UIEvent::AddEffect(key) => {
                spawn_effect_instance(world, assets, key);
            }
            UIEvent::InsertScalarKey {
                property_type,
                value,
            } => {
                let Some((clip_id, _, _)) = resolve_scalar_clip(world, assets) else {
                    continue;
                };
                let current_time = world
                    .get_resource::<TimelineState>()
                    .map(|t| t.current_time)
                    .unwrap_or(0.0);
                edit_clip(world, clip_id, "Scalar key edit", |clip| {
                    scalar_clip_insert_key(clip, *property_type, current_time, *value);
                });
            }
            UIEvent::InsertScalarKeyAtPlayhead { property_type } => {
                let Some((clip_id, entity, domain)) = resolve_scalar_clip(world, assets) else {
                    continue;
                };
                let current_time = world
                    .get_resource::<TimelineState>()
                    .map(|t| t.current_time)
                    .unwrap_or(0.0);
                let Some(value) = (domain.read)(world, entity, *property_type) else {
                    continue;
                };
                edit_clip(world, clip_id, "Scalar key edit", |clip| {
                    scalar_clip_insert_key(clip, *property_type, current_time, value);
                });
            }
            UIEvent::InsertScalarDebugKeys { seed } => {
                let Some((clip_id, entity, domain)) = resolve_scalar_clip(world, assets) else {
                    continue;
                };
                edit_clip(world, clip_id, "Scalar debug keys", |clip| {
                    scalar_clip_insert_debug_keys(
                        clip,
                        domain,
                        *seed,
                        crate::ecs::systems::TIMELINE_FALLBACK_DURATION_SECONDS,
                    );
                });
                extend_instance_to_clip_duration(world, entity, clip_id);
            }
            UIEvent::ClearScalarKeys => {
                let Some(clip_id) = existing_scalar_clip(world) else {
                    continue;
                };
                edit_clip(world, clip_id, "Scalar keys clear", |clip| {
                    scalar_clip_clear_keys(clip);
                });
            }
            UIEvent::ClipSetMinDuration { source_id, seconds } => {
                let seconds = seconds.max(0.0);
                edit_clip(world, *source_id, "Clip length edit", |clip| {
                    clip.min_duration = seconds;
                    crate::animation::editable::clip_recalculate_duration(clip);
                });
                sync_instances_to_clip_duration(world, *source_id);
            }
            UIEvent::OpenScalarCurveEditor => {
                let Some((clip_id, _, _)) = resolve_scalar_clip(world, assets) else {
                    continue;
                };
                if let Some(mut timeline) = world.get_resource_mut::<TimelineState>() {
                    timeline.current_clip_id = Some(clip_id);
                    timeline.selected_keyframes.clear();
                }
                let scalar_props: Vec<_> = world
                    .get_resource::<ClipLibrary>()
                    .and_then(|lib| {
                        lib.get(clip_id).map(|clip| {
                            clip.scalar_curves.iter().map(|c| c.property_type).collect()
                        })
                    })
                    .unwrap_or_default();
                if let Some(mut editor) = world.get_resource_mut::<CurveEditorState>() {
                    editor.is_open = true;
                    editor.needs_focus = true;
                    editor.select_scalars();
                    for prop in scalar_props {
                        editor.visible_curves.insert(prop);
                    }
                    editor.view_initialized = false;
                }
            }
            _ => continue,
        }
    }
}

fn resolve_scalar_clip(
    world: &mut World,
    assets: &mut AssetStorage,
) -> Option<(
    SourceClipId,
    crate::ecs::world::Entity,
    &'static crate::ecs::component::ScalarChannelDomain,
)> {
    let (entity, domain) = resolve_selected_scalar_entity(world)?;
    let clip_id = ensure_entity_clip(world, assets, entity, domain);
    Some((clip_id, entity, domain))
}

fn extend_instance_to_clip_duration(
    world: &mut World,
    entity: crate::ecs::world::Entity,
    clip_id: SourceClipId,
) {
    let Some(duration) = world
        .get_resource::<ClipLibrary>()
        .and_then(|lib| lib.get(clip_id).map(|clip| clip.duration))
    else {
        return;
    };
    if let Some(schedule) = world.get_component_mut::<crate::ecs::component::ClipSchedule>(entity) {
        for inst in schedule
            .instances
            .iter_mut()
            .filter(|i| i.source_id == clip_id)
        {
            inst.clip_out = inst.clip_out.max(duration);
        }
    }
}

/// Every schedule instance of the clip spans the whole clip again, so a length
/// edit shows up on the timeline blocks and in the loop extent (undoable per schedule).
fn sync_instances_to_clip_duration(world: &mut World, clip_id: SourceClipId) {
    let Some(duration) = world
        .get_resource::<ClipLibrary>()
        .and_then(|lib| lib.get(clip_id).map(|clip| clip.duration))
    else {
        return;
    };
    for entity in world.component_entities::<crate::ecs::component::ClipSchedule>() {
        let Some(before) = world
            .get_component::<crate::ecs::component::ClipSchedule>(entity)
            .filter(|schedule| schedule.instances.iter().any(|i| i.source_id == clip_id))
            .cloned()
        else {
            continue;
        };
        let mut after = before.clone();
        for inst in after
            .instances
            .iter_mut()
            .filter(|i| i.source_id == clip_id)
        {
            inst.clip_out = duration;
            inst.clip_in = inst.clip_in.min(duration);
        }
        world.insert_component(entity, after.clone());
        if let Some(mut history) = world.get_resource_mut::<EditHistory>() {
            history.push_schedule_edit(entity, before, after, "Clip length edit");
        }
    }
}

fn existing_scalar_clip(world: &World) -> Option<SourceClipId> {
    let (entity, _) = resolve_selected_scalar_entity(world)?;
    crate::ecs::systems::scalar_clip_systems::find_entity_clip_id(world, entity)
}

fn edit_clip(
    world: &mut World,
    clip_id: SourceClipId,
    description: &'static str,
    apply: impl FnOnce(&mut crate::animation::editable::EditableAnimationClip),
) {
    let (before, after) = {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        let Some(before) = clip_library.get(clip_id).cloned() else {
            return;
        };
        let Some(clip) = clip_library.get_mut(clip_id) else {
            return;
        };
        apply(clip);
        (before, clip.clone())
    };

    if let Some(mut history) = world.get_resource_mut::<EditHistory>() {
        crate::ecs::systems::edit_history_systems::edit_history_push_clip_mergeable(
            &mut history,
            clip_id,
            before,
            after,
            description,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::component::{FlameEffect, FlameParam};
    use crate::ecs::systems::phases::event_dispatch::edit_history::dispatch_edit_history_events;
    use crate::ecs::systems::scalar_clip_systems::find_entity_clip_id;
    use crate::ecs::systems::scalar_clip_systems::test_support::{
        spawn_probe, spawn_probe_with_clip, PROBE_DOMAIN, PROBE_HEIGHT, PROBE_LEVEL,
    };
    use crate::hooks::effect_spawn::EffectSpawnHooks;
    use crate::scene::test_support::ProbeOwner;

    fn make_world_with_probe() -> (World, AssetStorage) {
        let mut world = World::new();
        spawn_probe(&mut world, "Probe");
        world.insert_resource(ClipLibrary::new());
        world.insert_resource(TimelineState::new());
        world.insert_resource(EditHistory::new(10));
        world.insert_resource(EffectSpawnHooks::collect().expect("spawn hooks"));
        world.insert_resource(crate::hooks::pick::PickHooks::collect().expect("pick hooks"));
        (world, AssetStorage::new())
    }

    #[test]
    fn test_insert_key_creates_clip_and_schedule() {
        let (mut world, mut assets) = make_world_with_probe();
        let entity = world.entities_with::<ProbeOwner>()[0];

        dispatch_scalar_clip_events(
            &[UIEvent::InsertScalarKey {
                property_type: PROBE_LEVEL.property_type(),
                value: 2.5,
            }],
            &mut world,
            &mut assets,
        );

        let clip_id = find_entity_clip_id(&world, entity).expect("probe clip scheduled");
        let lib = world.get_resource::<ClipLibrary>().unwrap();
        let clip = lib.get(clip_id).expect("clip registered");
        let curve = clip
            .get_scalar_curve(PROBE_LEVEL.property_type())
            .expect("scalar curve");
        assert_eq!(curve.keyframes.len(), 1);
        assert!((curve.keyframes[0].value - 2.5).abs() < 1e-6);
    }

    #[test]
    fn test_insert_then_undo_restores_empty_clip() {
        let (mut world, mut assets) = make_world_with_probe();
        let entity = world.entities_with::<ProbeOwner>()[0];

        dispatch_scalar_clip_events(
            &[UIEvent::InsertScalarKey {
                property_type: PROBE_LEVEL.property_type(),
                value: 2.5,
            }],
            &mut world,
            &mut assets,
        );
        let clip_id = find_entity_clip_id(&world, entity).unwrap();

        dispatch_edit_history_events(&[UIEvent::Undo], &mut world);
        {
            let lib = world.get_resource::<ClipLibrary>().unwrap();
            assert!(lib.get(clip_id).unwrap().scalar_curves.is_empty());
        }

        dispatch_edit_history_events(&[UIEvent::Redo], &mut world);
        let lib = world.get_resource::<ClipLibrary>().unwrap();
        let clip = lib.get(clip_id).unwrap();
        assert_eq!(clip.scalar_curves.len(), 1);
    }

    #[test]
    fn test_insert_key_at_playhead_uses_current_component_value_and_time() {
        let (mut world, mut assets) = make_world_with_probe();
        let entity = world.entities_with::<ProbeOwner>()[0];

        world.resource_mut::<TimelineState>().current_time = 2.0;
        world
            .get_component_mut::<ProbeOwner>(entity)
            .unwrap()
            .position[1] = 4.25;

        dispatch_scalar_clip_events(
            &[UIEvent::InsertScalarKeyAtPlayhead {
                property_type: PROBE_HEIGHT.property_type(),
            }],
            &mut world,
            &mut assets,
        );

        let clip_id = find_entity_clip_id(&world, entity).expect("probe clip scheduled");
        let lib = world.get_resource::<ClipLibrary>().unwrap();
        let curve = lib
            .get(clip_id)
            .unwrap()
            .get_scalar_curve(PROBE_HEIGHT.property_type())
            .expect("curve created from empty clip");
        assert_eq!(curve.keyframes.len(), 1);
        assert!((curve.keyframes[0].time - 2.0).abs() < 1e-6);
        assert!((curve.keyframes[0].value - 4.25).abs() < 1e-6);
    }

    #[test]
    fn test_add_effect_creates_clip_and_schedule_by_default() {
        let (mut world, mut assets) = make_world_with_probe();

        dispatch_scalar_clip_events(&[UIEvent::AddEffect("probe")], &mut world, &mut assets);

        let probes = world.entities_with::<ProbeOwner>();
        assert_eq!(probes.len(), 2);
        let new_probe = probes[1];
        let clip_id =
            find_entity_clip_id(&world, new_probe).expect("new probe has a scheduled clip");
        let lib = world.get_resource::<ClipLibrary>().unwrap();
        let clip = lib.get(clip_id).expect("clip registered");
        assert_eq!(clip.name, PROBE_DOMAIN.name);
        assert!(clip.scalar_curves.is_empty());
    }

    #[test]
    fn test_set_min_duration_lengthens_unkeyed_clip_and_its_instance() {
        let (mut world, mut assets) = make_world_with_probe();
        dispatch_scalar_clip_events(&[UIEvent::AddEffect("probe")], &mut world, &mut assets);
        let probe = world.entities_with::<ProbeOwner>()[1];
        let clip_id = find_entity_clip_id(&world, probe).expect("clip scheduled");

        dispatch_scalar_clip_events(
            &[UIEvent::ClipSetMinDuration {
                source_id: clip_id,
                seconds: 12.0,
            }],
            &mut world,
            &mut assets,
        );

        let lib = world.get_resource::<ClipLibrary>().unwrap();
        let clip = lib.get(clip_id).expect("clip registered");
        assert!(clip.scalar_curves.is_empty());
        assert!((clip.duration - 12.0).abs() < 1e-6);
        let schedule = world
            .get_component::<crate::ecs::component::ClipSchedule>(probe)
            .expect("schedule");
        assert!((schedule.instances[0].clip_out - 12.0).abs() < 1e-6);
        assert!((schedule.instances[0].end_time() - 12.0).abs() < 1e-6);
    }

    #[test]
    fn test_spawn_with_clip_schedules_each_entity_separately() {
        let mut world = World::new();
        world.insert_resource(ClipLibrary::new());
        let mut assets = AssetStorage::new();

        let a = spawn_probe_with_clip(&mut world, &mut assets, "Probe");
        let b = spawn_probe_with_clip(&mut world, &mut assets, "Probe 2");

        let clip_a = find_entity_clip_id(&world, a).unwrap();
        let clip_b = find_entity_clip_id(&world, b).unwrap();
        assert_ne!(clip_a, clip_b, "each entity owns its own clip");
    }
}
