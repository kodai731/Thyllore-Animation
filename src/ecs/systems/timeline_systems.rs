use crate::animation::editable::SourceClipId;
use crate::ecs::resource::ClipLibrary;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::TimelineState;

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
