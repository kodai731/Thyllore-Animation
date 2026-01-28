use crate::animation::editable::{EditableClipId, EditableClipManager};
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{AnimationPlayback, TimelineState};

pub fn timeline_process_events(
    events: &[UIEvent],
    timeline_state: &mut TimelineState,
    playback: &mut AnimationPlayback,
    clip_manager: &mut EditableClipManager,
) {
    for event in events {
        match event {
            UIEvent::TimelinePlay => {
                timeline_state.playing = true;
                playback.playing = true;
            }

            UIEvent::TimelinePause => {
                timeline_state.playing = false;
                playback.playing = false;
            }

            UIEvent::TimelineStop => {
                timeline_state.playing = false;
                timeline_state.current_time = 0.0;
                playback.playing = false;
                playback.time = 0.0;
            }

            UIEvent::TimelineSetTime(time) => {
                timeline_state.playing = false;
                timeline_state.set_time(*time);
                playback.playing = false;
                playback.time = *time;
            }

            UIEvent::TimelineSetSpeed(speed) => {
                timeline_state.speed = *speed;
                playback.speed = *speed;
            }

            UIEvent::TimelineToggleLoop => {
                timeline_state.looping = !timeline_state.looping;
                playback.looping = timeline_state.looping;
            }

            UIEvent::TimelineSelectClip(clip_id) => {
                timeline_select_clip(timeline_state, playback, clip_manager, *clip_id);
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
                    if let Some(clip) = clip_manager.get_mut(clip_id) {
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
    playback: &mut AnimationPlayback,
    clip_manager: &EditableClipManager,
    clip_id: EditableClipId,
) {
    if let Some(clip) = clip_manager.get(clip_id) {
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

    if let Some(playable_clip) = clip_manager.to_playable_clip(clip_id) {
        playback.current_clip_id = Some(playable_clip.id);
        playback.time = 0.0;
    }
}

pub fn timeline_update(
    timeline_state: &mut TimelineState,
    playback: &mut AnimationPlayback,
    clip_manager: &EditableClipManager,
    delta_time: f32,
) {
    if !timeline_state.playing {
        return;
    }

    let duration = timeline_state
        .current_clip_id
        .and_then(|id| clip_manager.get(id))
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
        playback.playing = false;
    } else {
        timeline_state.current_time = new_time;
    }

    playback.time = timeline_state.current_time;
}

pub fn timeline_sync_from_playback(
    timeline_state: &mut TimelineState,
    playback: &AnimationPlayback,
) {
    timeline_state.current_time = playback.time;
    timeline_state.playing = playback.playing;
    timeline_state.looping = playback.looping;
    timeline_state.speed = playback.speed;
}
