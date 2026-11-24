use crate::asset::AssetStorage;
use crate::ecs::component::ClipSchedule;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::{
    AssetEditCommand, AssetEditQueue, ClipLibrary, ClipOrigin, ClipRevision, EditCommand,
    EditCommandAfter, EditEntry, EditHistory, RemovedClip, ScaleHistory, SkeletonSnapshot,
    ToastMessage, UiToast,
};
use crate::ecs::systems::{delete_library_clip, restore_deleted_clip};
use crate::ecs::world::{Entity, World};
use thyllore_anim_core::editable::{EditableAnimationClip, SourceClip};
use thyllore_vulkan_core::resource::GraphicsResources;

#[derive(Clone, Debug)]
pub enum EditHistoryEvent {
    Undo,
    Redo,
}

impl UiCommand for EditHistoryEvent {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        match *self {
            EditHistoryEvent::Undo => dispatch_undo(world),
            EditHistoryEvent::Redo => dispatch_redo(world, assets),
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
                    .revision(*clip_id)
                    .unwrap_or_else(|| before.clone());
                clip_library.restore_revision(*clip_id, before.clone());
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
            log!("Undo: {}", description);
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
            log!("Undo: {}", description);
        }

        EditCommand::ClipAdded {
            clip_id,
            description,
        } => {
            let removed_source = world
                .resource_mut::<ClipLibrary>()
                .remove(*clip_id)
                .map(|removed| removed.source);

            let redo_entry = EditEntry {
                command: EditCommand::ClipAdded {
                    clip_id: *clip_id,
                    description,
                },
                after: match removed_source {
                    Some(source) => EditCommandAfter::ClipCreated(source),
                    None => entry.after.clone(),
                },
            };

            world.resource_mut::<EditHistory>().push_to_redo(redo_entry);
            log!("Undo: {}", description);
        }

        EditCommand::ClipRemoved {
            clip_id,
            removed,
            effects,
            description,
        } => {
            restore_deleted_clip(world, *clip_id, removed, effects);
            world
                .resource_mut::<EditHistory>()
                .push_to_redo(entry.clone());
            log!("Undo: {}", description);
        }

        EditCommand::SkeletonModified {
            before,
            description,
        } => {
            if !matches!(entry.after, EditCommandAfter::Skeleton(_)) {
                return;
            }
            queue_skeleton_restore(world, before);
            world
                .resource_mut::<EditHistory>()
                .push_to_redo(entry.clone());
            log!("Undo: {}", description);
        }

        EditCommand::ModelScaled {
            factor,
            root_scales,
            description,
        } => {
            queue_model_scale(
                world,
                1.0 / factor,
                ScaleHistory::Revert {
                    root_scales: root_scales.clone(),
                },
            );
            world
                .resource_mut::<EditHistory>()
                .push_to_redo(entry.clone());
            log!("Undo: {}", description);
        }
    }

    show_history_toast(world, "Undo", entry.command.description());
}

fn dispatch_redo(world: &mut World, assets: &mut AssetStorage) {
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
            redo_clip_modified(world, *clip_id, after_clip, &entry.after, description);
        }

        (
            EditCommand::ScheduleModified {
                entity,
                description,
                ..
            },
            EditCommandAfter::Schedule(after_schedule),
        ) => {
            redo_schedule_modified(world, *entity, after_schedule, &entry.after, description);
        }

        (
            EditCommand::ClipAdded {
                clip_id,
                description,
            },
            EditCommandAfter::ClipCreated(source),
        ) => {
            redo_clip_added(world, *clip_id, source, description);
        }

        (
            EditCommand::ClipRemoved {
                clip_id,
                description,
                ..
            },
            EditCommandAfter::Empty,
        ) => {
            redo_clip_removed(world, assets, *clip_id, description);
        }

        (EditCommand::SkeletonModified { description, .. }, EditCommandAfter::Skeleton(after)) => {
            queue_skeleton_restore(world, after);
            world
                .resource_mut::<EditHistory>()
                .push_to_undo(entry.clone());
            log!("Redo: {}", description);
        }

        (
            EditCommand::ModelScaled {
                factor,
                description,
                ..
            },
            EditCommandAfter::Empty,
        ) => {
            queue_model_scale(world, *factor, ScaleHistory::Replay);
            world
                .resource_mut::<EditHistory>()
                .push_to_undo(entry.clone());
            log!("Redo: {}", description);
        }

        _ => return,
    }

    show_history_toast(world, "Redo", entry.command.description());
}

/// A skeleton restore rewrites assets and skin data, so it is queued for `App` like the edit itself.
fn queue_skeleton_restore(world: &mut World, target: &SkeletonSnapshot) {
    world
        .resource_mut::<AssetEditQueue>()
        .push(AssetEditCommand::RestoreSkeletonSnapshot {
            snapshot: Box::new(target.clone()),
        });
}

fn queue_model_scale(world: &mut World, factor: f32, history: ScaleHistory) {
    world
        .resource_mut::<AssetEditQueue>()
        .push(AssetEditCommand::ApplyModelScale { factor, history });
}

fn show_history_toast(world: &World, action: &str, description: &str) {
    world.resource_mut::<UiToast>().message =
        Some(ToastMessage::new(format!("{action}: {description}")));
}

fn redo_clip_modified(
    world: &mut World,
    clip_id: u64,
    after_clip: &EditableAnimationClip,
    after: &EditCommandAfter,
    description: &'static str,
) {
    let current = {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        let current = clip_library
            .revision(clip_id)
            .unwrap_or_else(|| ClipRevision {
                clip: after_clip.clone(),
                origin: ClipOrigin::User,
            });
        if let Some(clip) = clip_library.get_mut(clip_id) {
            *clip = after_clip.clone();
        }
        current
    };

    let undo_entry = EditEntry {
        command: EditCommand::ClipModified {
            clip_id,
            before: current,
            description,
        },
        after: after.clone(),
    };

    world.resource_mut::<EditHistory>().push_to_undo(undo_entry);
    log!("Redo: {}", description);
}

fn redo_schedule_modified(
    world: &mut World,
    entity: Entity,
    after_schedule: &ClipSchedule,
    after: &EditCommandAfter,
    description: &'static str,
) {
    let current = world
        .get_component::<ClipSchedule>(entity)
        .cloned()
        .unwrap_or_else(|| after_schedule.clone());
    if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
        *schedule = after_schedule.clone();
    }

    let undo_entry = EditEntry {
        command: EditCommand::ScheduleModified {
            entity,
            before: current,
            description,
        },
        after: after.clone(),
    };

    world.resource_mut::<EditHistory>().push_to_undo(undo_entry);
    log!("Redo: {}", description);
}

fn redo_clip_added(
    world: &mut World,
    clip_id: u64,
    source: &SourceClip,
    description: &'static str,
) {
    world.resource_mut::<ClipLibrary>().restore(
        clip_id,
        RemovedClip {
            source: source.clone(),
            origin: ClipOrigin::User,
        },
    );

    let undo_entry = EditEntry {
        command: EditCommand::ClipAdded {
            clip_id,
            description,
        },
        after: EditCommandAfter::ClipCreated(source.clone()),
    };

    world.resource_mut::<EditHistory>().push_to_undo(undo_entry);
    log!("Redo: {}", description);
}

fn redo_clip_removed(
    world: &mut World,
    assets: &mut AssetStorage,
    clip_id: u64,
    description: &'static str,
) {
    let Some((removed, effects)) = delete_library_clip(world, assets, clip_id) else {
        return;
    };

    let undo_entry = EditEntry {
        command: EditCommand::ClipRemoved {
            clip_id,
            removed,
            effects,
            description,
        },
        after: EditCommandAfter::Empty,
    };

    world.resource_mut::<EditHistory>().push_to_undo(undo_entry);
    log!("Redo: {}", description);
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn undo_of_a_model_scale_reverts_the_assets_and_the_root_scales() {
        let mut world = World::new();
        world.insert_resource(EditHistory::new(8));
        world.insert_resource(AssetEditQueue::default());
        world.insert_resource(UiToast::default());
        let root = world.spawn();
        let root_scales = vec![(root, Vec3::new(0.01, 0.01, 0.01))];
        world.resource_mut::<EditHistory>().push_model_scale(
            0.01,
            root_scales.clone(),
            "Apply model scale",
        );

        dispatch_undo(&mut world);

        let queued = world.resource_mut::<AssetEditQueue>().take();
        let [AssetEditCommand::ApplyModelScale { factor, history }] = queued.as_slice() else {
            panic!("expected one ApplyModelScale, got {queued:?}");
        };
        assert!((factor - 100.0).abs() < 1e-3);
        assert_eq!(*history, ScaleHistory::Revert { root_scales });
    }

    #[test]
    fn undo_of_the_first_edit_gives_a_model_clip_back_to_the_model() {
        let mut world = World::new();
        let mut assets = AssetStorage::default();
        world.insert_resource(EditHistory::new(8));
        world.insert_resource(UiToast::default());
        let mut library = ClipLibrary::new();
        let clip_id = crate::ecs::systems::clip_library_register_and_activate(
            &mut library,
            &mut assets,
            EditableAnimationClip::new(0, "walk".to_string()),
        );
        library.model_clip_ids.insert(clip_id);
        let before = library.revision(clip_id).expect("revision");
        library.get_mut(clip_id).expect("clip").duration = 2.0;
        let after = library.get(clip_id).cloned().expect("clip");
        world.insert_resource(library);
        world
            .resource_mut::<EditHistory>()
            .push_clip_edit(clip_id, before, after, "set duration");

        dispatch_undo(&mut world);
        let origin_after_undo = world.resource::<ClipLibrary>().origin(clip_id);
        dispatch_redo(&mut world, &mut assets);

        let library = world.resource::<ClipLibrary>();
        assert_eq!(origin_after_undo, ClipOrigin::Model);
        assert_eq!(library.origin(clip_id), ClipOrigin::User);
        assert_eq!(library.get(clip_id).expect("clip").duration, 2.0);
    }

    #[test]
    fn redo_of_a_model_scale_replays_the_bake() {
        let mut world = World::new();
        world.insert_resource(EditHistory::new(8));
        world.insert_resource(AssetEditQueue::default());
        world.insert_resource(UiToast::default());
        world
            .resource_mut::<EditHistory>()
            .push_model_scale(0.01, Vec::new(), "Apply model scale");
        dispatch_undo(&mut world);
        world.resource_mut::<AssetEditQueue>().take();

        dispatch_redo(&mut world, &mut AssetStorage::default());

        let queued = world.resource_mut::<AssetEditQueue>().take();
        let [AssetEditCommand::ApplyModelScale { factor, history }] = queued.as_slice() else {
            panic!("expected one ApplyModelScale, got {queued:?}");
        };
        assert!((factor - 0.01).abs() < 1e-6);
        assert_eq!(*history, ScaleHistory::Replay);
    }
}
