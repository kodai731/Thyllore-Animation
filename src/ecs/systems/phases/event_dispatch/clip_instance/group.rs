use crate::ecs::component::ClipSchedule;
use crate::ecs::systems::clip_schedule_systems::{
    clip_schedule_add_to_group, clip_schedule_create_group, clip_schedule_remove_from_group,
    clip_schedule_remove_group,
};
use crate::ecs::systems::timeline::clip_instance::modify_clip_group;
use crate::ecs::world::{Entity, World};

use super::ClipInstanceEvent;

pub(super) fn apply_clip_group_events(events: &[ClipInstanceEvent], world: &mut World) {
    for event in events {
        apply_clip_group_event(event, world);
    }
}

fn apply_clip_group_event(event: &ClipInstanceEvent, world: &mut World) {
    match event {
        ClipInstanceEvent::GroupCreate { entity, name } => {
            modify_clip_schedule(world, *entity, |schedule| {
                clip_schedule_create_group(schedule, name.clone());
            })
        }

        ClipInstanceEvent::GroupDelete { entity, group_id } => {
            modify_clip_schedule(world, *entity, |schedule| {
                clip_schedule_remove_group(schedule, *group_id)
            })
        }

        ClipInstanceEvent::GroupAddInstance {
            entity,
            group_id,
            instance_id,
        } => modify_clip_schedule(world, *entity, |schedule| {
            clip_schedule_add_to_group(schedule, *group_id, *instance_id)
        }),

        ClipInstanceEvent::GroupRemoveInstance {
            entity,
            group_id,
            instance_id,
        } => modify_clip_schedule(world, *entity, |schedule| {
            clip_schedule_remove_from_group(schedule, *group_id, *instance_id)
        }),

        ClipInstanceEvent::GroupToggleMute { entity, group_id } => {
            modify_clip_group(world, *entity, *group_id, |group| {
                group.muted = !group.muted
            })
        }

        ClipInstanceEvent::GroupSetWeight {
            entity,
            group_id,
            weight,
        } => modify_clip_group(world, *entity, *group_id, |group| {
            group.weight = weight.clamp(0.0, 1.0)
        }),

        _ => {}
    }
}

fn modify_clip_schedule(world: &mut World, entity: Entity, edit: impl FnOnce(&mut ClipSchedule)) {
    if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
        edit(schedule);
    }
}
