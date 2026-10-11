use std::collections::HashSet;

use crate::animation::editable::{BlendMode, ClipGroupId, ClipInstanceId, SourceClipId};
use crate::asset::AssetStorage;
use crate::ecs::component::ClipSchedule;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::EditHistory;
use crate::ecs::world::{Entity, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::apply_clip_instance_events;

#[derive(Clone, Debug)]
pub enum ClipInstanceEvent {
    Select {
        entity: Entity,
        instance_id: ClipInstanceId,
    },
    Deselect,
    Move {
        entity: Entity,
        instance_id: ClipInstanceId,
        new_start_time: f32,
    },
    TrimStart {
        entity: Entity,
        instance_id: ClipInstanceId,
        new_clip_in: f32,
    },
    TrimEnd {
        entity: Entity,
        instance_id: ClipInstanceId,
        new_clip_out: f32,
    },
    ToggleMute {
        entity: Entity,
        instance_id: ClipInstanceId,
    },
    Delete {
        entity: Entity,
        instance_id: ClipInstanceId,
    },
    SetWeight {
        entity: Entity,
        instance_id: ClipInstanceId,
        weight: f32,
    },
    SetBlendMode {
        entity: Entity,
        instance_id: ClipInstanceId,
        blend_mode: BlendMode,
    },
    GroupCreate {
        entity: Entity,
        name: String,
    },
    GroupDelete {
        entity: Entity,
        group_id: ClipGroupId,
    },
    GroupAddInstance {
        entity: Entity,
        group_id: ClipGroupId,
        instance_id: ClipInstanceId,
    },
    GroupRemoveInstance {
        entity: Entity,
        group_id: ClipGroupId,
        instance_id: ClipInstanceId,
    },
    GroupToggleMute {
        entity: Entity,
        group_id: ClipGroupId,
    },
    GroupSetWeight {
        entity: Entity,
        group_id: ClipGroupId,
        weight: f32,
    },
    Add {
        entity: Entity,
        source_id: SourceClipId,
        start_time: f32,
    },
}

impl ClipInstanceEvent {
    /// The entity whose schedule the event edits; selection and adds are not recorded in the history.
    fn edited_entity(&self) -> Option<Entity> {
        match self {
            ClipInstanceEvent::Move { entity, .. }
            | ClipInstanceEvent::TrimStart { entity, .. }
            | ClipInstanceEvent::TrimEnd { entity, .. }
            | ClipInstanceEvent::ToggleMute { entity, .. }
            | ClipInstanceEvent::Delete { entity, .. }
            | ClipInstanceEvent::SetWeight { entity, .. }
            | ClipInstanceEvent::SetBlendMode { entity, .. }
            | ClipInstanceEvent::GroupCreate { entity, .. }
            | ClipInstanceEvent::GroupDelete { entity, .. }
            | ClipInstanceEvent::GroupAddInstance { entity, .. }
            | ClipInstanceEvent::GroupRemoveInstance { entity, .. }
            | ClipInstanceEvent::GroupToggleMute { entity, .. }
            | ClipInstanceEvent::GroupSetWeight { entity, .. } => Some(*entity),
            ClipInstanceEvent::Select { .. }
            | ClipInstanceEvent::Deselect
            | ClipInstanceEvent::Add { .. } => None,
        }
    }
}

impl UiCommand for ClipInstanceEvent {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        let events = [*self];
        let snapshots = collect_clip_schedule_snapshots(&events, world);
        apply_clip_instance_events(&events, world);
        record_schedule_changes(snapshots, world);
    }
}

fn collect_clip_schedule_snapshots(
    events: &[ClipInstanceEvent],
    world: &World,
) -> Vec<(Entity, ClipSchedule)> {
    let entities: HashSet<Entity> = events
        .iter()
        .filter_map(ClipInstanceEvent::edited_entity)
        .collect();

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
    let Some(mut edit_history) = world.get_resource_mut::<EditHistory>() else {
        return;
    };

    for (entity, before) in snapshots {
        let Some(after) = world.get_component::<ClipSchedule>(entity).cloned() else {
            continue;
        };
        if before != after {
            edit_history.push_schedule_edit(entity, before, after, "clip schedule edit");
        }
    }
}
