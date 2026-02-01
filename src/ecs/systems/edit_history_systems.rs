use crate::ecs::component::ClipSchedule;
use crate::ecs::resource::{
    ClipLibrary, EditCommand, EditCommandAfter, EditEntry, EditHistory,
};
use crate::ecs::world::World;

pub fn apply_undo(
    edit_history: &mut EditHistory,
    clip_library: &mut ClipLibrary,
    world: &mut World,
) {
    let Some(entry) = edit_history.pop_undo() else {
        return;
    };

    match &entry.command {
        EditCommand::ClipModified {
            clip_id,
            before,
            description,
        } => {
            let current = clip_library
                .get(*clip_id)
                .cloned()
                .unwrap_or_else(|| before.clone());

            if let Some(clip) = clip_library.get_mut(*clip_id) {
                *clip = before.clone();
            }

            let redo_entry = EditEntry {
                command: EditCommand::ClipModified {
                    clip_id: *clip_id,
                    before: current,
                    description,
                },
                after: entry.after.clone(),
            };
            edit_history.push_to_redo(redo_entry);

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

            if let Some(schedule) =
                world.get_component_mut::<ClipSchedule>(*entity)
            {
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
            edit_history.push_to_redo(redo_entry);

            crate::log!("Undo: {}", description);
        }
    }
}

pub fn apply_redo(
    edit_history: &mut EditHistory,
    clip_library: &mut ClipLibrary,
    world: &mut World,
) {
    let Some(entry) = edit_history.pop_redo() else {
        return;
    };

    match (&entry.command, &entry.after) {
        (
            EditCommand::ClipModified {
                clip_id,
                description,
                ..
            },
            EditCommandAfter::Clip(after_clip),
        ) => {
            let current = clip_library
                .get(*clip_id)
                .cloned()
                .unwrap_or_else(|| after_clip.clone());

            if let Some(clip) = clip_library.get_mut(*clip_id) {
                *clip = after_clip.clone();
            }

            let undo_entry = EditEntry {
                command: EditCommand::ClipModified {
                    clip_id: *clip_id,
                    before: current,
                    description,
                },
                after: entry.after.clone(),
            };
            edit_history.push_to_undo(undo_entry);

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

            if let Some(schedule) =
                world.get_component_mut::<ClipSchedule>(*entity)
            {
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
            edit_history.push_to_undo(undo_entry);

            crate::log!("Redo: {}", description);
        }

        _ => {}
    }
}
