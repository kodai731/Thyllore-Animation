use crate::animation::editable::SourceClipId;
use crate::ecs::component::ClipSchedule;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{ClipLibrary, TimelineState};
use crate::ecs::world::World;

pub fn timeline_process_events(
    events: &[UIEvent],
    timeline_state: &mut TimelineState,
    clip_library: &mut ClipLibrary,
) {
    for event in events {
        match event {
            UIEvent::TimelinePlay => {
                timeline_state.playing = true;
            }

            UIEvent::TimelinePause => {
                timeline_state.playing = false;
            }

            UIEvent::TimelineStop => {
                timeline_state.playing = false;
                timeline_state.current_time = 0.0;
            }

            UIEvent::TimelineSetTime(time) => {
                timeline_state.playing = false;
                timeline_state.set_time(*time);
            }

            UIEvent::TimelineSetSpeed(speed) => {
                timeline_state.speed = *speed;
            }

            UIEvent::TimelineToggleLoop => {
                timeline_state.looping = !timeline_state.looping;
            }

            UIEvent::TimelineSelectClip(clip_id) => {
                timeline_select_clip(timeline_state, clip_library, *clip_id);
            }

            UIEvent::TimelineToggleTrack(bone_id) => {
                timeline_state.toggle_track_expanded(*bone_id);
            }

            UIEvent::TimelineExpandTrack(bone_id) => {
                timeline_state.expand_track(*bone_id);
            }

            UIEvent::TimelineCollapseTrack(bone_id) => {
                timeline_state.collapse_track(*bone_id);
            }

            UIEvent::TimelineSelectKeyframe {
                bone_id,
                property_type,
                keyframe_id,
            } => {
                use crate::ecs::resource::SelectedKeyframe;
                let selected = SelectedKeyframe::new(*bone_id, *property_type, *keyframe_id);
                timeline_state.select_keyframe(selected);
            }

            UIEvent::TimelineAddKeyframe { .. } => {}

            UIEvent::TimelineDeleteSelectedKeyframes => {}

            UIEvent::TimelineMoveKeyframe {
                bone_id,
                property_type,
                keyframe_id,
                new_time,
                new_value,
            } => {
                if let Some(clip_id) = timeline_state.current_clip_id {
                    if let Some(clip) = clip_library.get_mut(clip_id) {
                        if let Some(track) = clip.tracks.get_mut(bone_id) {
                            track.move_keyframe(*property_type, *keyframe_id, *new_time, *new_value);
                        }
                    }
                }
            }

            UIEvent::TimelineZoomIn => {
                timeline_state.zoom_in();
            }

            UIEvent::TimelineZoomOut => {
                timeline_state.zoom_out();
            }

            _ => {}
        }
    }
}

fn timeline_select_clip(
    timeline_state: &mut TimelineState,
    clip_library: &ClipLibrary,
    clip_id: SourceClipId,
) {
    if let Some(clip) = clip_library.get(clip_id) {
        timeline_state.current_clip_id = Some(clip_id);
        timeline_state.current_time = 0.0;
        timeline_state.selected_keyframes.clear();
        timeline_state.expanded_tracks.clear();

        if let Some((&first_bone_id, _)) = clip.tracks.iter().next() {
            timeline_state.expand_track(first_bone_id);
        }

        crate::log!(
            "Timeline: Selected clip '{}' (id={}, duration={:.2}s, tracks={})",
            clip.name,
            clip_id,
            clip.duration,
            clip.track_count()
        );
    }
}

pub fn timeline_update(
    timeline_state: &mut TimelineState,
    clip_library: &ClipLibrary,
    delta_time: f32,
) {
    if !timeline_state.playing {
        return;
    }

    let duration = timeline_state
        .current_clip_id
        .and_then(|id| clip_library.get(id))
        .map(|c| c.duration)
        .unwrap_or(0.0);

    if duration <= 0.0 {
        return;
    }

    let new_time = timeline_state.current_time + delta_time * timeline_state.speed;

    if timeline_state.looping {
        timeline_state.current_time = new_time % duration;
    } else if new_time >= duration {
        timeline_state.current_time = duration;
        timeline_state.playing = false;
    } else {
        timeline_state.current_time = new_time;
    }
}

pub fn process_clip_instance_events(events: &[UIEvent], world: &mut World) {
    let mut deselect_after: Option<(crate::ecs::world::Entity, crate::animation::editable::ClipInstanceId)> = None;

    for event in events {
        match event {
            UIEvent::ClipInstanceSelect { entity, instance_id } => {
                let mut ts = world.resource_mut::<TimelineState>();
                ts.selected_clip_instance = Some((*entity, *instance_id));
            }

            UIEvent::ClipInstanceDeselect => {
                let mut ts = world.resource_mut::<TimelineState>();
                ts.selected_clip_instance = None;
            }

            UIEvent::ClipInstanceMove { entity, instance_id, new_start_time } => {
                modify_clip_instance(world, *entity, *instance_id, |inst| {
                    inst.start_time = *new_start_time;
                });
            }

            UIEvent::ClipInstanceTrimStart { entity, instance_id, new_clip_in } => {
                modify_clip_instance(world, *entity, *instance_id, |inst| {
                    inst.clip_in = new_clip_in.max(0.0);
                });
            }

            UIEvent::ClipInstanceTrimEnd { entity, instance_id, new_clip_out } => {
                modify_clip_instance(world, *entity, *instance_id, |inst| {
                    inst.clip_out = new_clip_out.max(0.0);
                });
            }

            UIEvent::ClipInstanceToggleMute { entity, instance_id } => {
                modify_clip_instance(world, *entity, *instance_id, |inst| {
                    inst.muted = !inst.muted;
                });
            }

            UIEvent::ClipInstanceDelete { entity, instance_id } => {
                if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                    schedule.remove_instance(*instance_id);
                }
                deselect_after = Some((*entity, *instance_id));
            }

            UIEvent::ClipInstanceSetWeight { entity, instance_id, weight } => {
                modify_clip_instance(world, *entity, *instance_id, |inst| {
                    inst.weight = weight.clamp(0.0, 1.0);
                });
            }

            UIEvent::ClipInstanceSetBlendMode { entity, instance_id, blend_mode } => {
                modify_clip_instance(world, *entity, *instance_id, |inst| {
                    inst.blend_mode = *blend_mode;
                });
            }

            UIEvent::ClipGroupCreate { entity, name } => {
                if let Some(schedule) =
                    world.get_component_mut::<ClipSchedule>(*entity)
                {
                    schedule.create_group(name.clone());
                }
            }

            UIEvent::ClipGroupDelete { entity, group_id } => {
                if let Some(schedule) =
                    world.get_component_mut::<ClipSchedule>(*entity)
                {
                    schedule.remove_group(*group_id);
                }
            }

            UIEvent::ClipGroupAddInstance { entity, group_id, instance_id } => {
                if let Some(schedule) =
                    world.get_component_mut::<ClipSchedule>(*entity)
                {
                    schedule.add_instance_to_group(*group_id, *instance_id);
                }
            }

            UIEvent::ClipGroupRemoveInstance { entity, group_id, instance_id } => {
                if let Some(schedule) =
                    world.get_component_mut::<ClipSchedule>(*entity)
                {
                    schedule.remove_instance_from_group(*group_id, *instance_id);
                }
            }

            UIEvent::ClipGroupToggleMute { entity, group_id } => {
                if let Some(schedule) =
                    world.get_component_mut::<ClipSchedule>(*entity)
                {
                    if let Some(group) =
                        schedule.groups.iter_mut().find(|g| g.id == *group_id)
                    {
                        group.muted = !group.muted;
                    }
                }
            }

            UIEvent::ClipGroupSetWeight { entity, group_id, weight } => {
                if let Some(schedule) =
                    world.get_component_mut::<ClipSchedule>(*entity)
                {
                    if let Some(group) =
                        schedule.groups.iter_mut().find(|g| g.id == *group_id)
                    {
                        group.weight = weight.clamp(0.0, 1.0);
                    }
                }
            }

            _ => {}
        }
    }

    if let Some((entity, instance_id)) = deselect_after {
        let mut ts = world.resource_mut::<TimelineState>();
        if let Some((sel_entity, sel_id)) = ts.selected_clip_instance {
            if sel_entity == entity && sel_id == instance_id {
                ts.selected_clip_instance = None;
            }
        }
    }
}

fn modify_clip_instance(
    world: &mut World,
    entity: crate::ecs::world::Entity,
    instance_id: crate::animation::editable::ClipInstanceId,
    f: impl FnOnce(&mut crate::animation::editable::ClipInstance),
) {
    if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
        if let Some(inst) = schedule.instances.iter_mut().find(|i| i.instance_id == instance_id) {
            f(inst);
        }
    }
}
