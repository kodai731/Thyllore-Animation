use crate::asset::AssetStorage;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{ClipLibrary, EditHistory, PoseLibrary, TimelineState};
use crate::ecs::systems::pose_library_systems::{apply_pose_to_clip, capture_current_pose};
use crate::ecs::world::World;

pub fn dispatch_pose_library_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &mut AssetStorage,
) {
    for event in events {
        match event {
            UIEvent::PoseLibrarySaveCurrent { name } => {
                let clip_library = world.resource::<ClipLibrary>();
                let timeline_state = world.resource::<TimelineState>();
                let current_clip_id = timeline_state.current_clip_id;
                let current_time = timeline_state.current_time;
                drop(timeline_state);

                if let Some(pose) =
                    capture_current_pose(name, &clip_library, current_clip_id, current_time)
                {
                    drop(clip_library);
                    let mut clip_library = world.resource_mut::<ClipLibrary>();
                    let pose_id =
                        crate::ecs::systems::clip_library_systems::clip_library_register_and_activate(
                            &mut clip_library,
                            assets,
                            pose,
                        );

                    let source = clip_library.source_clips.get(&pose_id).cloned();
                    drop(clip_library);

                    let mut pose_library = world.resource_mut::<PoseLibrary>();
                    pose_library.add_pose(pose_id, current_time);
                    pose_library.selected_pose_id = Some(pose_id);

                    if let Some(source) = source {
                        if world.contains_resource::<EditHistory>() {
                            let mut edit_history = world.resource_mut::<EditHistory>();
                            edit_history.push_clip_added(pose_id, source, "save pose");
                        }
                    }

                    log!(
                        "Saved pose '{}' (id={}) at time {:.3}",
                        name,
                        pose_id,
                        current_time
                    );
                }
            }

            UIEvent::PoseLibraryApply(pose_id) => {
                let timeline_state = world.resource::<TimelineState>();
                let target_clip_id = timeline_state.current_clip_id;
                let target_time = timeline_state.current_time;
                drop(timeline_state);

                let clip_library = world.resource::<ClipLibrary>();
                let pose_clip = clip_library.get(*pose_id).cloned();
                drop(clip_library);

                if let (Some(pose), Some(target_id)) = (pose_clip, target_clip_id) {
                    let mut clip_library = world.resource_mut::<ClipLibrary>();
                    let before = clip_library.get(target_id).cloned();

                    if let Some(target) = clip_library.get_mut(target_id) {
                        apply_pose_to_clip(&pose, target, target_time);
                    }

                    let after = clip_library.get(target_id).cloned();
                    drop(clip_library);

                    if let (Some(before), Some(after)) = (before, after) {
                        if world.contains_resource::<EditHistory>() {
                            let mut edit_history = world.resource_mut::<EditHistory>();
                            edit_history.push_clip_edit(target_id, before, after, "apply pose");
                        }
                    }

                    log!("Applied pose (id={}) at time {:.3}", pose_id, target_time);
                }
            }

            UIEvent::PoseLibraryDelete(pose_id) => {
                let removed_source = {
                    let clip_library = world.resource::<ClipLibrary>();
                    clip_library.source_clips.get(pose_id).cloned()
                };

                let mut pose_library = world.resource_mut::<PoseLibrary>();
                pose_library.remove_pose(*pose_id);

                let mut clip_library = world.resource_mut::<ClipLibrary>();
                clip_library.remove(*pose_id);
                drop(clip_library);

                if let Some(removed) = removed_source {
                    if world.contains_resource::<EditHistory>() {
                        let mut edit_history = world.resource_mut::<EditHistory>();
                        edit_history.push_clip_removed(*pose_id, removed, "delete pose");
                    }
                }

                log!("Deleted pose (id={})", pose_id);
            }

            _ => {}
        }
    }
}
