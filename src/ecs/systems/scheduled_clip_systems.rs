use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

use super::scalar_clip_systems::{find_entity_clip_id, schedule_entity_clip};
use crate::asset::AssetStorage;
use crate::ecs::component::ClipSchedule;
use crate::ecs::resource::ClipLibrary;
use crate::ecs::world::{Entity, World};
use crate::hooks::scene::{
    decode_component, encode_scene_value, entities_with, SceneComponentHook, SceneComponentRole,
    SceneValue,
};
use crate::scene::find_scene_clip_file;
use thyllore_anim_core::editable::{EditableAnimationClip, SourceClipId};

/// The clip an entity schedules: found by its file in `animation_clips`, by name when it was never saved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledClip {
    pub clip: String,
    #[serde(default)]
    pub file: Option<String>,
}

impl SceneComponent for ScheduledClip {
    const TYPE_KEY: &'static str = "clip";
    const PERSISTED_FIELDS: &'static [&'static str] = &["clip", "file"];
}

inventory::submit! { SCHEDULED_CLIP_SCENE_COMPONENT }

const SCHEDULED_CLIP_SCENE_COMPONENT: SceneComponentHook = SceneComponentHook {
    type_key: ScheduledClip::TYPE_KEY,
    role: SceneComponentRole::Attachment,
    entities: entities_with::<ClipSchedule>,
    capture: capture_scheduled_clip,
    apply: apply_scheduled_clip,
};

fn capture_scheduled_clip(world: &World, entity: Entity) -> Option<SceneValue> {
    let clip_id = find_entity_clip_id(world, entity)?;
    let library = world.get_resource::<ClipLibrary>()?;
    if library.is_model_clip(clip_id) {
        return None;
    }

    let clip = library.get(clip_id)?;
    encode_scene_value(&ScheduledClip {
        clip: clip.name.clone(),
        file: find_scene_clip_file(world, clip),
    })
    .ok()
}

fn apply_scheduled_clip(
    world: &mut World,
    _assets: &mut AssetStorage,
    entity: Entity,
    value: &SceneValue,
) -> anyhow::Result<()> {
    let scheduled: ScheduledClip = decode_component(value)?;
    match find_unscheduled_clip(world, &scheduled) {
        Some(clip_id) => schedule_entity_clip(world, entity, clip_id),
        None => log_warn!(
            "Scene references clip {} which is not loaded; an empty clip is scheduled instead",
            scheduled.clip
        ),
    }
    Ok(())
}

/// Lowest-id library clip no entity schedules yet: the one from the saved file, otherwise by name.
fn find_unscheduled_clip(world: &World, scheduled: &ScheduledClip) -> Option<SourceClipId> {
    let taken_clip_ids: BTreeSet<SourceClipId> = world
        .iter_components::<ClipSchedule>()
        .flat_map(|(_, schedule)| schedule.instances.iter().map(|i| i.source_id))
        .collect();
    let library = world.get_resource::<ClipLibrary>()?;
    let find_free_clip = |is_scheduled_clip: &dyn Fn(&EditableAnimationClip) -> bool| {
        library
            .source_clips
            .iter()
            .filter(|(id, _)| !taken_clip_ids.contains(id))
            .filter(|(_, source)| is_scheduled_clip(&source.editable_clip))
            .map(|(id, _)| *id)
            .min()
    };

    let clip_from_file = scheduled.file.as_ref().and_then(|scene_file| {
        find_free_clip(&|clip| find_scene_clip_file(world, clip).as_ref() == Some(scene_file))
    });
    clip_from_file.or_else(|| find_free_clip(&|clip| clip.name == scheduled.clip))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::systems::clip_library_register_and_activate;

    fn world_with_clips_named_take(
        clip_files: &[&str],
    ) -> (World, AssetStorage, Vec<SourceClipId>) {
        let mut world = World::new();
        let mut assets = AssetStorage::new();
        let mut library = ClipLibrary::new();
        let clip_ids = clip_files
            .iter()
            .map(|clip_file| {
                let mut clip = EditableAnimationClip::new(0, "take".to_string());
                clip.source_path = Some(clip_file.to_string());
                clip_library_register_and_activate(&mut library, &mut assets, clip)
            })
            .collect();
        world.insert_resource(library);

        (world, assets, clip_ids)
    }

    #[test]
    fn clips_sharing_a_name_are_told_apart_by_their_file() {
        let (mut world, mut assets, clip_ids) = world_with_clips_named_take(&[
            "assets/animations/take.anim.ron",
            "assets/animations/take_2.anim.ron",
        ]);
        let entity = world.spawn();
        schedule_entity_clip(&mut world, entity, clip_ids[1]);
        let saved = capture_scheduled_clip(&world, entity).expect("scheduled clip");
        let restored_entity = world.spawn();

        world.remove_component::<ClipSchedule>(entity);
        apply_scheduled_clip(&mut world, &mut assets, restored_entity, &saved).unwrap();

        assert_eq!(
            find_entity_clip_id(&world, restored_entity),
            Some(clip_ids[1])
        );
    }

    #[test]
    fn unsaved_clip_files_of_one_name_are_told_apart_by_their_directory() {
        let (mut world, mut assets, clip_ids) = world_with_clips_named_take(&[
            "thyllore_walk_clips/take.anim.ron",
            "thyllore_run_clips/take.anim.ron",
        ]);
        let entity = world.spawn();
        schedule_entity_clip(&mut world, entity, clip_ids[1]);
        let saved = capture_scheduled_clip(&world, entity).expect("scheduled clip");
        let restored_entity = world.spawn();

        world.remove_component::<ClipSchedule>(entity);
        apply_scheduled_clip(&mut world, &mut assets, restored_entity, &saved).unwrap();

        assert_eq!(
            find_entity_clip_id(&world, restored_entity),
            Some(clip_ids[1])
        );
    }

    #[test]
    fn clip_without_a_file_is_found_by_its_name() {
        let (world, mut assets, _) = world_with_clips_named_take(&[]);
        let unsaved_clip_id = clip_library_register_and_activate(
            &mut world.resource_mut::<ClipLibrary>(),
            &mut assets,
            EditableAnimationClip::new(0, "unsaved".to_string()),
        );
        let scheduled = ScheduledClip {
            clip: "unsaved".to_string(),
            file: None,
        };

        assert_eq!(
            find_unscheduled_clip(&world, &scheduled),
            Some(unsaved_clip_id)
        );
    }
}
