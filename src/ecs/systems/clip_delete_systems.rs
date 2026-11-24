use crate::asset::AssetStorage;
use crate::ecs::component::{scalar_domain_for_entity, ClipSchedule};
use crate::ecs::resource::{ClipDeletionEffects, ClipLibrary, RemovedClip, TimelineState};
use crate::ecs::systems::timeline::timeline_select_clip;
use crate::ecs::systems::{
    clip_library_register_empty_model_clip, clip_schedule_add_instance, select_first_animated_clip,
};
use crate::ecs::world::{Entity, World};
use thyllore_anim_core::editable::SourceClipId;

/// Deletes a clip even while schedules play it; `None` for a clip the library does not hold.
pub fn delete_library_clip(
    world: &mut World,
    assets: &mut AssetStorage,
    clip_id: SourceClipId,
) -> Option<(RemovedClip, ClipDeletionEffects)> {
    let selected_clip = world.resource::<TimelineState>().current_clip_id;
    let removed = world.resource_mut::<ClipLibrary>().remove(clip_id)?;
    let schedules = release_clip_from_schedules(world, clip_id);
    let mut effects = ClipDeletionEffects {
        schedules,
        selected_clip,
        created_clip: None,
    };

    let was_selected = selected_clip == Some(clip_id);
    let models_without_clip = find_models_without_clip(world, &effects.schedules);
    if !was_selected && models_without_clip.is_empty() {
        return Some((removed, effects));
    }

    let scalar_clip_ids = find_scalar_clip_ids(world);
    let kept_selection = selected_clip
        .filter(|_| !was_selected)
        .filter(|selected| !scalar_clip_ids.contains(selected));
    let successor = kept_selection.or_else(|| find_model_clip_candidate(world, &scalar_clip_ids));
    let successor = match successor {
        Some(successor) => Some(successor),
        None if models_without_clip.is_empty() => None,
        None => {
            let created = clip_library_register_empty_model_clip(
                &mut world.resource_mut::<ClipLibrary>(),
                assets,
            );
            effects.created_clip = Some(created);
            Some(created)
        }
    };

    if was_selected {
        select_successor_clip(world, successor);
    }
    if let Some(successor) = successor {
        schedule_clip_on(world, &models_without_clip, successor);
    }

    Some((removed, effects))
}

/// Puts back what `delete_library_clip` took: the clip, the schedules that played it and the selection.
pub fn restore_deleted_clip(
    world: &mut World,
    clip_id: SourceClipId,
    removed: &RemovedClip,
    effects: &ClipDeletionEffects,
) {
    {
        let mut library = world.resource_mut::<ClipLibrary>();
        if let Some(created_clip) = effects.created_clip {
            library.remove(created_clip);
        }
        library.restore(clip_id, removed.clone());
    }

    for (entity, schedule_before) in &effects.schedules {
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
            *schedule = schedule_before.clone();
        }
    }

    if effects.selected_clip == Some(clip_id) {
        select_successor_clip(world, Some(clip_id));
    }
}

fn release_clip_from_schedules(
    world: &mut World,
    clip_id: SourceClipId,
) -> Vec<(Entity, ClipSchedule)> {
    let mut schedules_before = Vec::new();

    for entity in world.component_entities::<ClipSchedule>() {
        let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) else {
            continue;
        };
        if !schedule.instances.iter().any(|i| i.source_id == clip_id) {
            continue;
        }

        schedules_before.push((entity, schedule.clone()));
        schedule.instances.retain(|i| i.source_id != clip_id);
    }

    schedules_before
}

/// Scalar-domain entities play their own per-entity clip and may stay without one.
fn find_models_without_clip(world: &World, released: &[(Entity, ClipSchedule)]) -> Vec<Entity> {
    released
        .iter()
        .map(|(entity, _)| *entity)
        .filter(|entity| scalar_domain_for_entity(world, *entity).is_none())
        .filter(|entity| {
            world
                .get_component::<ClipSchedule>(*entity)
                .is_some_and(|schedule| schedule.instances.is_empty())
        })
        .collect()
}

/// A clip a scalar-domain entity plays holds that entity's curves, so a model never takes it over.
fn find_scalar_clip_ids(world: &World) -> Vec<SourceClipId> {
    world
        .iter_components::<ClipSchedule>()
        .filter(|(entity, _)| scalar_domain_for_entity(world, *entity).is_some())
        .flat_map(|(_, schedule)| schedule.instances.iter().map(|i| i.source_id))
        .collect()
}

/// The first animated clip no scalar-domain entity plays, see `select_first_animated_clip`.
fn find_model_clip_candidate(
    world: &World,
    scalar_clip_ids: &[SourceClipId],
) -> Option<SourceClipId> {
    let library = world.resource::<ClipLibrary>();
    let mut candidate_ids: Vec<SourceClipId> = library
        .all_clip_ids()
        .copied()
        .filter(|id| !scalar_clip_ids.contains(id))
        .collect();
    candidate_ids.sort_unstable();

    select_first_animated_clip(&library, &candidate_ids)
}

fn select_successor_clip(world: &mut World, successor: Option<SourceClipId>) {
    let library = world.resource::<ClipLibrary>();
    let mut timeline = world.resource_mut::<TimelineState>();

    timeline.current_clip_id = None;
    if let Some(successor) = successor {
        timeline_select_clip(&mut timeline, &library, successor);
    }
}

fn schedule_clip_on(world: &mut World, entities: &[Entity], clip_id: SourceClipId) {
    let duration = world
        .resource::<ClipLibrary>()
        .get(clip_id)
        .map_or(1.0, |clip| clip.duration);

    for entity in entities {
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
            clip_schedule_add_instance(schedule, clip_id, duration);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::systems::scalar_clip_systems::test_support::spawn_probe_with_clip;
    use crate::ecs::systems::{
        clip_library_register_and_activate, find_entity_clip_id, schedule_entity_clip,
    };
    use thyllore_anim_core::editable::EditableAnimationClip;

    fn world_with_model() -> (World, AssetStorage, Entity) {
        let mut world = World::new();
        world.insert_resource(ClipLibrary::new());
        world.insert_resource(TimelineState::default());
        let model = world.spawn();
        (world, AssetStorage::new(), model)
    }

    fn register_clip(world: &mut World, assets: &mut AssetStorage, name: &str) -> SourceClipId {
        clip_library_register_and_activate(
            &mut world.resource_mut::<ClipLibrary>(),
            assets,
            EditableAnimationClip::new(0, name.to_string()),
        )
    }

    fn select_and_schedule(world: &mut World, model: Entity, clip_id: SourceClipId) {
        schedule_entity_clip(world, model, clip_id);
        world.resource_mut::<TimelineState>().current_clip_id = Some(clip_id);
    }

    fn selected_clip(world: &World) -> Option<SourceClipId> {
        world.resource::<TimelineState>().current_clip_id
    }

    #[test]
    fn deleting_the_selected_clip_moves_the_model_to_another_clip() {
        let (mut world, mut assets, model) = world_with_model();
        let kept_clip = register_clip(&mut world, &mut assets, "kept");
        let deleted_clip = register_clip(&mut world, &mut assets, "deleted");
        select_and_schedule(&mut world, model, deleted_clip);

        let (_, effects) =
            delete_library_clip(&mut world, &mut assets, deleted_clip).expect("deleted");

        assert!(world.resource::<ClipLibrary>().get(deleted_clip).is_none());
        assert_eq!(selected_clip(&world), Some(kept_clip));
        assert_eq!(find_entity_clip_id(&world, model), Some(kept_clip));
        assert_eq!(effects.created_clip, None);
    }

    #[test]
    fn deleting_the_last_clip_gives_the_model_an_empty_clip() {
        let (mut world, mut assets, model) = world_with_model();
        let only_clip = register_clip(&mut world, &mut assets, "only");
        select_and_schedule(&mut world, model, only_clip);

        let (_, effects) =
            delete_library_clip(&mut world, &mut assets, only_clip).expect("deleted");

        let created_clip = effects.created_clip.expect("an empty clip was created");
        assert_eq!(selected_clip(&world), Some(created_clip));
        assert_eq!(find_entity_clip_id(&world, model), Some(created_clip));
        assert_eq!(world.resource::<ClipLibrary>().clip_count(), 1);
    }

    #[test]
    fn restoring_a_deleted_clip_puts_back_the_schedule_and_the_selection() {
        let (mut world, mut assets, model) = world_with_model();
        let only_clip = register_clip(&mut world, &mut assets, "only");
        select_and_schedule(&mut world, model, only_clip);
        let (removed, effects) =
            delete_library_clip(&mut world, &mut assets, only_clip).expect("deleted");

        restore_deleted_clip(&mut world, only_clip, &removed, &effects);

        let library = world.resource::<ClipLibrary>();
        assert_eq!(library.clip_count(), 1);
        assert_eq!(
            library.get(only_clip).map(|c| c.name.as_str()),
            Some("only")
        );
        assert_eq!(selected_clip(&world), Some(only_clip));
        assert_eq!(find_entity_clip_id(&world, model), Some(only_clip));
    }

    #[test]
    fn deleting_an_unselected_clip_keeps_the_selection() {
        let (mut world, mut assets, model) = world_with_model();
        let selected = register_clip(&mut world, &mut assets, "selected");
        let unused_clip = register_clip(&mut world, &mut assets, "unused");
        select_and_schedule(&mut world, model, selected);

        let (_, effects) =
            delete_library_clip(&mut world, &mut assets, unused_clip).expect("deleted");

        assert_eq!(selected_clip(&world), Some(selected));
        assert_eq!(find_entity_clip_id(&world, model), Some(selected));
        assert!(effects.schedules.is_empty());
    }

    #[test]
    fn deleting_a_scalar_entity_clip_leaves_that_entity_without_a_clip() {
        let (mut world, mut assets, model) = world_with_model();
        let model_clip = register_clip(&mut world, &mut assets, "model");
        select_and_schedule(&mut world, model, model_clip);
        let probe = spawn_probe_with_clip(&mut world, &mut assets, "probe");
        let probe_clip = find_entity_clip_id(&world, probe).expect("probe clip");

        let (_, effects) =
            delete_library_clip(&mut world, &mut assets, probe_clip).expect("deleted");

        assert_eq!(find_entity_clip_id(&world, probe), None);
        assert_eq!(selected_clip(&world), Some(model_clip));
        assert_eq!(effects.created_clip, None);
    }

    #[test]
    fn deleted_selection_never_falls_on_a_scalar_entity_clip() {
        let (mut world, mut assets, model) = world_with_model();
        let probe = spawn_probe_with_clip(&mut world, &mut assets, "probe");
        let probe_clip = find_entity_clip_id(&world, probe).expect("probe clip");
        let model_clip = register_clip(&mut world, &mut assets, "model");
        select_and_schedule(&mut world, model, model_clip);

        let (_, effects) =
            delete_library_clip(&mut world, &mut assets, model_clip).expect("deleted");

        assert_ne!(selected_clip(&world), Some(probe_clip));
        assert_eq!(selected_clip(&world), effects.created_clip);
        assert_eq!(find_entity_clip_id(&world, probe), Some(probe_clip));
    }

    #[test]
    fn model_left_without_a_clip_never_takes_the_selected_scalar_entity_clip() {
        let (mut world, mut assets, model) = world_with_model();
        let model_clip = register_clip(&mut world, &mut assets, "model");
        schedule_entity_clip(&mut world, model, model_clip);
        let probe = spawn_probe_with_clip(&mut world, &mut assets, "probe");
        let probe_clip = find_entity_clip_id(&world, probe).expect("probe clip");
        world.resource_mut::<TimelineState>().current_clip_id = Some(probe_clip);

        let (_, effects) =
            delete_library_clip(&mut world, &mut assets, model_clip).expect("deleted");

        assert_eq!(find_entity_clip_id(&world, model), effects.created_clip);
        assert!(effects.created_clip.is_some());
        assert_eq!(selected_clip(&world), Some(probe_clip));
    }

    #[test]
    fn deleting_an_unknown_clip_changes_nothing() {
        let (mut world, mut assets, _) = world_with_model();

        assert!(delete_library_clip(&mut world, &mut assets, 42).is_none());
    }
}
