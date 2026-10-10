use crate::animation::editable::BlendMode;
use crate::animation::editable::ClipGroupId;
use crate::animation::editable::ClipInstanceId;
use crate::animation::editable::SourceClipId;
use crate::asset::AssetStorage;
use crate::ecs::component::ClipSchedule;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::EditHistory;
use crate::ecs::systems::process_clip_instance_events;
use crate::ecs::world::{Entity, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

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

impl UiCommand for ClipInstanceEvent {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        dispatch_clip_instance_events(&[*self], world);
    }
}

fn dispatch_clip_instance_events(events: &[ClipInstanceEvent], world: &mut World) {
    let schedule_snapshots = collect_clip_schedule_snapshots(events, world);

    process_clip_instance_events(events, world);

    record_schedule_changes(schedule_snapshots, world);
}

fn collect_clip_schedule_snapshots(
    events: &[ClipInstanceEvent],
    world: &World,
) -> Vec<(Entity, ClipSchedule)> {
    use std::collections::HashSet;

    let mut entities = HashSet::new();
    for event in events {
        match event {
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
            | ClipInstanceEvent::GroupSetWeight { entity, .. } => {
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
