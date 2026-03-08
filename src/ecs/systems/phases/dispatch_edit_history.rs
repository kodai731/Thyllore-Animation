use crate::ecs::component::ClipSchedule;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{ClipLibrary, EditCommand, EditCommandAfter, EditEntry, EditHistory};
use crate::ecs::world::World;

pub fn dispatch_edit_history_events(events: &[UIEvent], world: &mut World) {
    for event in events {
        match event {
            UIEvent::Undo => dispatch_undo(world),
            UIEvent::Redo => dispatch_redo(world),
            _ => {}
        }
    }
}

fn dispatch_undo(world: &mut World) {
    let entry = {
        let mut edit_history = world.resource_mut::<EditHistory>();
        if !edit_history.can_undo() {
            return;
        }
        edit_history.pop_undo()
    };
    let Some(entry) = entry else { return };

    match &entry.command {
        EditCommand::ClipModified {
            clip_id,
            before,
            description,
        } => {
            let current = {
                let mut clip_library = world.resource_mut::<ClipLibrary>();
                let current = clip_library
                    .get(*clip_id)
                    .cloned()
                    .unwrap_or_else(|| before.clone());
                if let Some(clip) = clip_library.get_mut(*clip_id) {
                    *clip = before.clone();
                }
                current
            };

            let redo_entry = EditEntry {
                command: EditCommand::ClipModified {
                    clip_id: *clip_id,
                    before: current,
                    description,
                },
                after: entry.after.clone(),
            };

            world.resource_mut::<EditHistory>().push_to_redo(redo_entry);
            crate::log!("Undo: {}", description);
        }

        EditCommand::ScheduleModified {
            entity,
            before,
            description,
        } => {
            let current = world
                .get_component::<ClipSchedule>(*entity)
                .cloned()
                .unwrap_or_else(|| before.clone());
            if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                *schedule = before.clone();
            }

            let redo_entry = EditEntry {
                command: EditCommand::ScheduleModified {
                    entity: *entity,
                    before: current,
                    description,
                },
                after: entry.after.clone(),
            };

            world.resource_mut::<EditHistory>().push_to_redo(redo_entry);
            crate::log!("Undo: {}", description);
        }
    }
}

fn dispatch_redo(world: &mut World) {
    let entry = {
        let mut edit_history = world.resource_mut::<EditHistory>();
        if !edit_history.can_redo() {
            return;
        }
        edit_history.pop_redo()
    };
    let Some(entry) = entry else { return };

    match (&entry.command, &entry.after) {
        (
            EditCommand::ClipModified {
                clip_id,
                description,
                ..
            },
            EditCommandAfter::Clip(after_clip),
        ) => {
            let current = {
                let mut clip_library = world.resource_mut::<ClipLibrary>();
                let current = clip_library
                    .get(*clip_id)
                    .cloned()
                    .unwrap_or_else(|| after_clip.clone());
                if let Some(clip) = clip_library.get_mut(*clip_id) {
                    *clip = after_clip.clone();
                }
                current
            };

            let undo_entry = EditEntry {
                command: EditCommand::ClipModified {
                    clip_id: *clip_id,
                    before: current,
                    description,
                },
                after: entry.after.clone(),
            };

            world.resource_mut::<EditHistory>().push_to_undo(undo_entry);
            crate::log!("Redo: {}", description);
        }

        (
            EditCommand::ScheduleModified {
                entity,
                description,
                ..
            },
            EditCommandAfter::Schedule(after_schedule),
        ) => {
            let current = world
                .get_component::<ClipSchedule>(*entity)
                .cloned()
                .unwrap_or_else(|| after_schedule.clone());
            if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                *schedule = after_schedule.clone();
            }

            let undo_entry = EditEntry {
                command: EditCommand::ScheduleModified {
                    entity: *entity,
                    before: current,
                    description,
                },
                after: entry.after.clone(),
            };

            world.resource_mut::<EditHistory>().push_to_undo(undo_entry);
            crate::log!("Redo: {}", description);
        }

        _ => {}
    }
}
