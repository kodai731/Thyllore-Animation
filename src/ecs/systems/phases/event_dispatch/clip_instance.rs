use crate::ecs::component::ClipSchedule;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::EditHistory;
use crate::ecs::systems::process_clip_instance_events;
use crate::ecs::world::{Entity, World};

pub fn dispatch_clip_instance_events(events: &[UIEvent], world: &mut World) {
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
