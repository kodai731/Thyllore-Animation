use crate::animation::editable::SourceClipId;
use crate::animation::BoneId;
use crate::ecs::resource::{
    ClipDragType, ClipLibrary, SelectedKeyframe, SelectionModifier, TimelineState,
};

pub fn timeline_select_clip(
    timeline_state: &mut TimelineState,
    clip_library: &ClipLibrary,
    clip_id: SourceClipId,
) {
    if timeline_state.current_clip_id == Some(clip_id) {
        return;
    }
    let Some(clip) = clip_library.get(clip_id) else {
        return;
    };

    timeline_state.current_clip_id = Some(clip_id);
    timeline_state.current_time = 0.0;
    timeline_state.selected_keyframes.clear();
    timeline_state.expanded_tracks.clear();
    if let Some((&first_bone_id, _)) = clip.tracks.iter().next() {
        timeline_state.expand_track(first_bone_id);
    }
    timeline_apply_fit_zoom(timeline_state, clip.duration);

    log!(
        "Timeline: Selected clip '{}' (id={}, duration={:.2}s, tracks={})",
        clip.name,
        clip_id,
        clip.duration,
        clip.track_count()
    );
}

pub fn timeline_apply_fit_zoom(timeline_state: &mut TimelineState, clip_duration: f32) {
    timeline_state.scroll_offset = 0.0;

    let visible_width = timeline_state.last_visible_width;
    if visible_width < 1.0 || clip_duration <= 0.0 {
        return;
    }

    let fit_zoom =
        visible_width / (clip_duration * crate::platform::ui::timeline_window::PIXELS_PER_SECOND);
    timeline_state.zoom_level = fit_zoom.clamp(0.01, 100.0);
}

pub fn timeline_apply_selection(
    state: &mut TimelineState,
    keyframe: SelectedKeyframe,
    modifier: SelectionModifier,
) {
    match modifier {
        SelectionModifier::Replace => {
            state.selected_keyframes.clear();
            state.selected_keyframes.insert(keyframe);
        }
        SelectionModifier::Add => {
            state.selected_keyframes.insert(keyframe);
        }
        SelectionModifier::Toggle => {
            if state.selected_keyframes.contains(&keyframe) {
                state.selected_keyframes.remove(&keyframe);
            } else {
                state.selected_keyframes.insert(keyframe);
            }
        }
    }
}

pub fn timeline_set_keyframe_selection(
    state: &mut TimelineState,
    keyframes: &[SelectedKeyframe],
    modifier: SelectionModifier,
) {
    if modifier == SelectionModifier::Replace {
        state.selected_keyframes.clear();
    }

    let per_keyframe_modifier = match modifier {
        SelectionModifier::Replace | SelectionModifier::Add => SelectionModifier::Add,
        SelectionModifier::Toggle => SelectionModifier::Toggle,
    };
    for keyframe in keyframes {
        timeline_apply_selection(state, keyframe.clone(), per_keyframe_modifier);
    }
}

pub fn timeline_toggle_track_expanded(state: &mut TimelineState, bone_id: BoneId) {
    if state.expanded_tracks.contains(&bone_id) {
        state.expanded_tracks.remove(&bone_id);
    } else {
        state.expanded_tracks.insert(bone_id);
    }
}

pub fn timeline_zoom_in(state: &mut TimelineState, max_zoom: f32) {
    state.zoom_level = (state.zoom_level * 1.2).min(max_zoom);
}

pub fn timeline_zoom_out(state: &mut TimelineState, min_zoom: f32) {
    state.zoom_level = (state.zoom_level / 1.2).max(min_zoom);
}

/// Timeline (start, end) a dragged clip block should be drawn at while the drag
/// is in progress. Mirrors the clamps the commit events apply on release
/// (`ClipInstanceMove`/`TrimStart`/`TrimEnd`), so the live preview always shows
/// exactly what releasing the mouse would produce.
pub fn clip_drag_preview_times(
    drag_type: &ClipDragType,
    original_value: f32,
    delta_time: f32,
    inst_start: f32,
    inst_end: f32,
    clip_in: f32,
    clip_out: f32,
) -> (f32, f32) {
    let span = clip_out - clip_in;
    let seconds_per_clip_second = if span > 1e-6 {
        (inst_end - inst_start) / span
    } else {
        1.0
    };

    match drag_type {
        ClipDragType::Move => {
            let start = (original_value + delta_time).max(0.0);
            (start, start + (inst_end - inst_start))
        }
        ClipDragType::TrimStart => {
            let new_clip_in = (original_value + delta_time).clamp(0.0, clip_out);
            (
                inst_start,
                inst_start + (clip_out - new_clip_in) * seconds_per_clip_second,
            )
        }
        ClipDragType::TrimEnd => {
            let new_clip_out = (original_value + delta_time).max(clip_in);
            (
                inst_start,
                inst_start + (new_clip_out - clip_in) * seconds_per_clip_second,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::{EditableAnimationClip, PropertyType, SourceClip};
    use crate::ecs::component::ClipSchedule;
    use crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance;
    use crate::ecs::systems::timeline::test_support::clip_with_three_keys;

    #[test]
    fn set_keyframe_selection_replace() {
        let (mut state, _) = clip_with_three_keys();
        let new_sel = vec![SelectedKeyframe::for_bone(0, PropertyType::ScaleX, 999)];

        timeline_set_keyframe_selection(&mut state, &new_sel, SelectionModifier::Replace);

        assert_eq!(state.selected_keyframes.len(), 1);
        assert!(state
            .selected_keyframes
            .contains(&SelectedKeyframe::for_bone(0, PropertyType::ScaleX, 999)));
    }

    #[test]
    fn set_keyframe_selection_add() {
        let (mut state, _) = clip_with_three_keys();
        let original_count = state.selected_keyframes.len();
        let new_sel = vec![SelectedKeyframe::for_bone(0, PropertyType::ScaleX, 999)];

        timeline_set_keyframe_selection(&mut state, &new_sel, SelectionModifier::Add);

        assert_eq!(state.selected_keyframes.len(), original_count + 1);
    }

    #[test]
    fn set_keyframe_selection_toggle() {
        let (mut state, _) = clip_with_three_keys();
        let existing = state.selected_keyframes.iter().next().unwrap().clone();
        let new_kf = SelectedKeyframe::for_bone(0, PropertyType::ScaleX, 999);
        let before_count = state.selected_keyframes.len();

        timeline_set_keyframe_selection(
            &mut state,
            &[existing.clone(), new_kf.clone()],
            SelectionModifier::Toggle,
        );

        assert_eq!(state.selected_keyframes.len(), before_count);
        assert!(!state.selected_keyframes.contains(&existing));
        assert!(state.selected_keyframes.contains(&new_kf));
    }

    #[test]
    fn drag_preview_trim_end_extends_zero_length_clip() {
        let (start, end) =
            clip_drag_preview_times(&ClipDragType::TrimEnd, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0);
        assert!((start - 0.0).abs() < 1e-6);
        assert!((end - 3.0).abs() < 1e-6);
    }

    #[test]
    fn drag_preview_mirrors_commit_clamps() {
        let (_, end) =
            clip_drag_preview_times(&ClipDragType::TrimEnd, 2.0, -5.0, 1.0, 3.0, 1.0, 2.0);
        assert!((end - 1.0).abs() < 1e-6);

        let (start, end) =
            clip_drag_preview_times(&ClipDragType::TrimStart, 0.5, 9.0, 1.0, 3.0, 0.5, 2.0);
        assert!((start - 1.0).abs() < 1e-6);
        assert!((end - 1.0).abs() < 1e-6);

        let (start, end) =
            clip_drag_preview_times(&ClipDragType::Move, 1.0, -5.0, 1.0, 3.0, 0.0, 2.0);
        assert!((start - 0.0).abs() < 1e-6);
        assert!((end - 2.0).abs() < 1e-6);
    }

    #[test]
    fn drag_preview_respects_playback_speed_scale() {
        let (start, end) =
            clip_drag_preview_times(&ClipDragType::TrimEnd, 1.0, 1.0, 1.0, 3.0, 0.0, 1.0);
        assert!((start - 1.0).abs() < 1e-6);
        assert!((end - 5.0).abs() < 1e-6);
    }

    #[test]
    fn select_same_clip_keeps_time() {
        let (mut state, library) = clip_with_three_keys();
        let clip_id = state.current_clip_id.unwrap();
        state.current_time = 0.7;

        timeline_select_clip(&mut state, &library, clip_id);

        assert!((state.current_time - 0.7).abs() < 1e-5);
    }

    #[test]
    fn select_clip_does_not_touch_schedule() {
        let (mut state, mut library) = clip_with_three_keys();
        let first_clip_id = state.current_clip_id.unwrap();

        let other_clip_id: SourceClipId = 99;
        let mut other_clip = EditableAnimationClip::new(other_clip_id, "other".to_string());
        other_clip.add_track(0, "bone0".to_string());
        library
            .source_clips
            .insert(other_clip_id, SourceClip::new(other_clip_id, other_clip));

        let mut schedule = ClipSchedule::new();
        clip_schedule_add_instance(&mut schedule, first_clip_id, 1.0);

        timeline_select_clip(&mut state, &library, other_clip_id);

        assert_eq!(state.current_clip_id, Some(other_clip_id));
        assert_eq!(schedule.instances.len(), 1);
        assert_eq!(schedule.instances[0].source_id, first_clip_id);
    }
}
