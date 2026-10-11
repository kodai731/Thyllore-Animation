use crate::ecs::resource::{ClipLibrary, TimelineState};
use crate::ecs::systems::timeline::keyframe_edit::{
    add_keyframe, edit_curve, insert_keyframe_preserving_shape, move_keyframe, remove_keyframe,
    remove_keyframes, set_extrapolation, shift_keyframes,
};

use super::TimelineEvent;

/// Applies the keyframe edits of the batch to the active clip; true when the clip changed.
pub(super) fn apply_keyframe_edits(
    events: &[TimelineEvent],
    timeline_state: &mut TimelineState,
    clip_library: &mut ClipLibrary,
) -> bool {
    let Some(clip_id) = timeline_state.current_clip_id else {
        return false;
    };

    let mut clip_modified = false;
    for event in events {
        let Some(clip) = clip_library.get_mut(clip_id) else {
            return clip_modified;
        };
        clip_modified |= match event {
            TimelineEvent::AddKeyframe {
                track,
                property_type,
                time,
                value,
            } => add_keyframe(clip, *track, *property_type, *time, *value),

            TimelineEvent::InsertKeyframeOnCurve {
                track,
                property_type,
                time,
            } => insert_keyframe_preserving_shape(clip, *track, *property_type, *time),

            TimelineEvent::MoveSelectedKeyframes { time_delta } => {
                shift_keyframes(clip, &timeline_state.selected_keyframes, *time_delta);
                true
            }

            TimelineEvent::DeleteSelectedKeyframes => {
                let had_selection = !timeline_state.selected_keyframes.is_empty();
                if had_selection {
                    remove_keyframes(clip, &timeline_state.selected_keyframes);
                }
                timeline_state.clear_selection();
                had_selection
            }

            TimelineEvent::MoveKeyframe {
                track,
                property_type,
                keyframe_id,
                new_time,
                new_value,
            } => {
                move_keyframe(
                    clip,
                    *track,
                    *property_type,
                    *keyframe_id,
                    *new_time,
                    *new_value,
                );
                true
            }

            TimelineEvent::DeleteKeyframe {
                track,
                property_type,
                keyframe_id,
            } => {
                remove_keyframe(clip, *track, *property_type, *keyframe_id);
                true
            }

            TimelineEvent::SetCurveExtrapolation {
                track,
                property_type,
                end,
                mode,
            } => edit_curve(clip_library, clip_id, *track, *property_type, |curve| {
                set_extrapolation(curve, *end, *mode)
            }),

            _ => false,
        };
    }

    clip_modified
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::{
        curve_add_keyframe, curve_sample, CurveExtrapolation, EditableAnimationClip, PropertyType,
        SourceClip,
    };
    use crate::animation::BoneId;
    use crate::ecs::resource::CurveTrackRef;
    use crate::ecs::systems::phases::event_dispatch::timeline::ExtrapolationEnd;
    use crate::ecs::systems::timeline::test_support::clip_with_three_keys;

    #[test]
    fn move_selected_keyframes_shifts_time() {
        let (mut state, mut library) = clip_with_three_keys();
        let events = vec![TimelineEvent::MoveSelectedKeyframes { time_delta: 0.25 }];

        let modified = apply_keyframe_edits(&events, &mut state, &mut library);
        assert!(modified);

        let clip = library.get(1).unwrap();
        let track = clip.tracks.get(&0).unwrap();
        let tx_curve = track.get_curve(PropertyType::TranslationX);
        let times: Vec<f32> = tx_curve.keyframes.iter().map(|k| k.time).collect();
        assert!(
            (times[0] - 0.75).abs() < 0.01,
            "Expected 0.75, got {}",
            times[0]
        );
        assert!(
            (times[1] - 1.25).abs() < 0.01,
            "Expected 1.25, got {}",
            times[1]
        );

        let ty_curve = track.get_curve(PropertyType::TranslationY);
        let ty_time = ty_curve.keyframes[0].time;
        assert!(
            (ty_time - 1.05).abs() < 0.01,
            "Expected 1.05, got {}",
            ty_time
        );
    }

    #[test]
    fn move_selected_keyframes_clamps_at_zero() {
        let (mut state, mut library) = clip_with_three_keys();
        let events = vec![TimelineEvent::MoveSelectedKeyframes { time_delta: -2.0 }];

        apply_keyframe_edits(&events, &mut state, &mut library);

        let clip = library.get(1).unwrap();
        let track = clip.tracks.get(&0).unwrap();
        for kf in &track.get_curve(PropertyType::TranslationX).keyframes {
            assert!(kf.time >= 0.0, "Time should be >= 0, got {}", kf.time);
        }
    }

    #[test]
    fn scalar_add_keyframe_rejects_bone_property_types() {
        let (mut state, mut library) = clip_with_three_keys();
        let events = vec![
            TimelineEvent::AddKeyframe {
                track: CurveTrackRef::Scalar,
                property_type: PropertyType::TranslationX,
                time: 0.5,
                value: 1.0,
            },
            TimelineEvent::AddKeyframe {
                track: CurveTrackRef::Scalar,
                property_type: PropertyType::Custom(0),
                time: 0.5,
                value: 1.0,
            },
        ];

        apply_keyframe_edits(&events, &mut state, &mut library);

        let clip = library.get(state.current_clip_id.unwrap()).unwrap();
        assert_eq!(clip.scalar_curves.len(), 1);
        assert_eq!(clip.scalar_curves[0].property_type, PropertyType::Custom(0));
    }

    #[test]
    fn insert_keyframe_on_curve_preserves_shape() {
        let (mut state, mut library) = clip_with_three_keys();
        let clip_id = state.current_clip_id.unwrap();
        let bone_id: BoneId = 0;
        let read_curve = |library: &ClipLibrary| {
            library.get(clip_id).unwrap().tracks[&bone_id]
                .get_curve(PropertyType::TranslationX)
                .clone()
        };
        let curve_before = read_curve(&library);

        let events = [TimelineEvent::InsertKeyframeOnCurve {
            track: CurveTrackRef::Bone(bone_id),
            property_type: PropertyType::TranslationX,
            time: 0.75,
        }];
        assert!(apply_keyframe_edits(&events, &mut state, &mut library));

        let curve_after = read_curve(&library);
        assert_eq!(
            curve_after.keyframe_count(),
            curve_before.keyframe_count() + 1
        );
        for time in [0.6, 0.7, 0.8, 0.9] {
            let before = curve_sample(&curve_before, time).unwrap();
            let after = curve_sample(&curve_after, time).unwrap();
            assert!(
                (before - after).abs() < 1e-5,
                "t={time}: {before} -> {after}"
            );
        }
    }

    #[test]
    fn set_curve_extrapolation_event() {
        let mut timeline_state = TimelineState::default();
        let mut clip_library = ClipLibrary::new();
        let mut clip = EditableAnimationClip::new(1, "test".to_string());
        let curve = clip.get_or_add_scalar_curve(PropertyType::Custom(7));
        curve_add_keyframe(curve, 0.5, 2.0);
        assert_eq!(curve.pre_extrapolation, CurveExtrapolation::Constant);
        assert_eq!(curve.post_extrapolation, CurveExtrapolation::Constant);
        clip_library
            .source_clips
            .insert(1, SourceClip::new(1, clip));
        timeline_state.current_clip_id = Some(1);

        let events = vec![TimelineEvent::SetCurveExtrapolation {
            track: CurveTrackRef::Scalar,
            property_type: PropertyType::Custom(7),
            end: ExtrapolationEnd::Pre,
            mode: CurveExtrapolation::Cycle,
        }];
        let modified = apply_keyframe_edits(&events, &mut timeline_state, &mut clip_library);
        assert!(modified);

        let curve = clip_library
            .get(1)
            .unwrap()
            .get_scalar_curve(PropertyType::Custom(7))
            .unwrap();
        assert_eq!(curve.pre_extrapolation, CurveExtrapolation::Cycle);
        assert_eq!(curve.post_extrapolation, CurveExtrapolation::Constant);
    }
}
