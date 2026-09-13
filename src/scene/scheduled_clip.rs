use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

use crate::animation::editable::SourceClipId;
use crate::asset::AssetStorage;
use crate::ecs::component::ClipSchedule;
use crate::ecs::resource::ClipLibrary;
use crate::ecs::systems::scalar_clip_systems::{find_entity_clip_id, schedule_entity_clip};
use crate::ecs::world::{Entity, World};
use crate::hooks::scene::{
    decode_component, encode_scene_value, entities_with, SceneComponentHook, SceneComponentRole,
    SceneValue,
};

/// The clip an entity schedules, named after the clip file the scene lists in `animation_clips`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledClip {
    pub clip: String,
}

impl SceneComponent for ScheduledClip {
    const TYPE_KEY: &'static str = "clip";
    const PERSISTED_FIELDS: &'static [&'static str] = &["clip"];
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
    let clip = library.get(clip_id)?;
    encode_scene_value(&ScheduledClip {
        clip: clip.name.clone(),
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
    match find_unscheduled_clip_named(world, &scheduled.clip) {
        Some(clip_id) => schedule_entity_clip(world, entity, clip_id),
        None => log_warn!(
            "Scene references clip {} which is not loaded; an empty clip is scheduled instead",
            scheduled.clip
        ),
    }
    Ok(())
}

/// Lowest-id library clip with that name that no entity schedules yet.
fn find_unscheduled_clip_named(world: &World, name: &str) -> Option<SourceClipId> {
    let scheduled: BTreeSet<SourceClipId> = world
        .iter_components::<ClipSchedule>()
        .flat_map(|(_, schedule)| schedule.instances.iter().map(|i| i.source_id))
        .collect();
    let library = world.get_resource::<ClipLibrary>()?;
    library
        .source_clips
        .iter()
        .filter(|(id, source)| source.editable_clip.name == name && !scheduled.contains(id))
        .map(|(id, _)| *id)
        .min()
}
