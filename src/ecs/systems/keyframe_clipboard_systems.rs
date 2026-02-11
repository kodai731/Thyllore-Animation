use crate::animation::editable::mirror::{build_mirror_mapping, mirror_keyframes};
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{ClipLibrary, CopiedKeyframe, KeyframeCopyBuffer, TimelineState};

pub fn process_keyframe_clipboard_events(
    events: &[UIEvent],
    timeline_state: &TimelineState,
    clip_library: &mut ClipLibrary,
    copy_buffer: &mut KeyframeCopyBuffer,
) {
    for event in events {
        match event {
            UIEvent::TimelineCopyKeyframes => {
                copy_keyframes(timeline_state, clip_library, copy_buffer);
            }

            UIEvent::TimelinePasteKeyframes { paste_time } => {
                paste_keyframes(*paste_time, timeline_state, clip_library, copy_buffer);
            }

            UIEvent::TimelineMirrorPaste { paste_time } => {
                mirror_paste_keyframes(*paste_time, timeline_state, clip_library, copy_buffer);
            }

            _ => {}
        }
    }
}

fn copy_keyframes(
    timeline_state: &TimelineState,
    clip_library: &ClipLibrary,
    copy_buffer: &mut KeyframeCopyBuffer,
) {
    let clip_id = match timeline_state.current_clip_id {
        Some(id) => id,
        None => return,
    };

    let clip = match clip_library.get(clip_id) {
        Some(c) => c,
        None => return,
    };

    if timeline_state.selected_keyframes.is_empty() {
        return;
    }

    copy_buffer.clear();
    copy_buffer.source_clip_id = Some(clip_id);

    let mut min_time = f32::MAX;
    let mut entries = Vec::new();

    for sel in &timeline_state.selected_keyframes {
        if let Some(track) = clip.tracks.get(&sel.bone_id) {
            let curve = track.get_curve(sel.property_type);
            if let Some(kf) = curve.get_keyframe(sel.keyframe_id) {
                if kf.time < min_time {
                    min_time = kf.time;
                }
                entries.push(CopiedKeyframe {
                    bone_id: sel.bone_id,
                    property_type: sel.property_type,
                    relative_time: kf.time,
                    value: kf.value,
                    interpolation: kf.interpolation,
                    in_tangent: kf.in_tangent.clone(),
                    out_tangent: kf.out_tangent.clone(),
                });
            }
        }
    }

    for entry in &mut entries {
        entry.relative_time -= min_time;
    }

    copy_buffer.base_time = min_time;
    copy_buffer.entries = entries;
}

fn paste_keyframes(
    paste_time: f32,
    timeline_state: &TimelineState,
    clip_library: &mut ClipLibrary,
    copy_buffer: &KeyframeCopyBuffer,
) {
    if copy_buffer.is_empty() {
        return;
    }

    let clip_id = match timeline_state.current_clip_id {
        Some(id) => id,
        None => return,
    };

    let clip = match clip_library.get_mut(clip_id) {
        Some(c) => c,
        None => return,
    };

    for entry in &copy_buffer.entries {
        let time = paste_time + entry.relative_time;
        if let Some(track) = clip.tracks.get_mut(&entry.bone_id) {
            let curve = track.get_curve_mut(entry.property_type);
            let new_id = curve.add_keyframe(time, entry.value);
            curve.set_keyframe_interpolation(new_id, entry.interpolation);
            curve.set_keyframe_tangents(
                new_id,
                entry.in_tangent.clone(),
                entry.out_tangent.clone(),
            );
        }
    }
}

fn mirror_paste_keyframes(
    paste_time: f32,
    timeline_state: &TimelineState,
    clip_library: &mut ClipLibrary,
    copy_buffer: &KeyframeCopyBuffer,
) {
    if copy_buffer.is_empty() {
        return;
    }

    let clip_id = match timeline_state.current_clip_id {
        Some(id) => id,
        None => return,
    };

    let bone_names = match clip_library.get(clip_id) {
        Some(clip) => clip
            .tracks
            .iter()
            .map(|(id, track)| (*id, track.bone_name.clone()))
            .collect(),
        None => return,
    };

    let mapping = build_mirror_mapping(&bone_names);
    let mirrored = mirror_keyframes(copy_buffer, &mapping);

    let clip = match clip_library.get_mut(clip_id) {
        Some(c) => c,
        None => return,
    };

    for entry in &mirrored.entries {
        let time = paste_time + entry.relative_time;
        if let Some(track) = clip.tracks.get_mut(&entry.bone_id) {
            let curve = track.get_curve_mut(entry.property_type);
            let new_id = curve.add_keyframe(time, entry.value);
            curve.set_keyframe_interpolation(new_id, entry.interpolation);
            curve.set_keyframe_tangents(
                new_id,
                entry.in_tangent.clone(),
                entry.out_tangent.clone(),
            );
        }
    }
}
