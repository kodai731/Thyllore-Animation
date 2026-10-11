use crate::animation::editable::{
    compute_tween, KeyframeId, PropertyCurve, PropertyType, TweenKind,
};
use crate::ecs::resource::{CurveEditorState, CurveSelectedKeyframe, CurveTrackRef, TweenSession};
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::world::World;

const TWEEN_BUTTONS: &[(&str, TweenKind)] = &[
    ("Blend", TweenKind::BlendToNeighbor),
    ("Breakdown", TweenKind::Breakdown),
    ("Smooth", TweenKind::Smooth),
];

pub(super) fn compute_curve_tween(
    curve: &PropertyCurve,
    selected_keyframes: &[CurveSelectedKeyframe],
    session: &TweenSession,
) -> Vec<(KeyframeId, f32)> {
    let selected_ids: Vec<KeyframeId> = selected_keyframes
        .iter()
        .filter(|selected| selected.property_type == curve.property_type)
        .map(|selected| selected.keyframe_id)
        .collect();
    compute_tween(curve, &selected_ids, session.kind, session.factor)
}

pub(super) fn curve_with_values(
    curve: &PropertyCurve,
    values: &[(KeyframeId, f32)],
) -> PropertyCurve {
    let mut replaced = curve.clone();
    for keyframe in &mut replaced.keyframes {
        if let Some((_, value)) = values.iter().find(|(id, _)| *id == keyframe.id) {
            keyframe.value = *value;
        }
    }
    replaced
}

pub(super) fn build_tween_controls(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &mut CurveEditorState,
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    track_ref: CurveTrackRef,
) {
    match editor_state.tween.as_mut() {
        Some(session) => {
            let min_factor = match session.kind {
                TweenKind::Smooth => 0.0,
                TweenKind::BlendToNeighbor | TweenKind::Breakdown => -1.0,
            };
            ui.slider("##tween", min_factor, 1.0, &mut session.factor);

            ui.same_line();
            if ui.small_button("Apply (Tab)") {
                apply_tween(world, editor_state, curves, track_ref);
            }
            ui.same_line();
            if ui.small_button("Cancel (Esc)") {
                editor_state.tween = None;
            }
        }
        None if !editor_state.selected_keyframes.is_empty() => {
            for (index, (label, kind)) in TWEEN_BUTTONS.iter().enumerate() {
                if index > 0 {
                    ui.same_line();
                }
                if ui.small_button(label) {
                    editor_state.tween = Some(TweenSession {
                        kind: *kind,
                        factor: 0.0,
                    });
                }
            }
        }
        None => {}
    }
}

pub(super) fn apply_tween(
    world: &World,
    editor_state: &mut CurveEditorState,
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    track_ref: CurveTrackRef,
) {
    let Some(session) = editor_state.tween.take() else {
        return;
    };

    for (curve, _, _) in curves {
        for (keyframe_id, new_value) in
            compute_curve_tween(curve, &editor_state.selected_keyframes, &session)
        {
            let Some(keyframe) = curve.get_keyframe(keyframe_id) else {
                continue;
            };
            world.send_command(TimelineEvent::MoveKeyframe {
                track: track_ref,
                property_type: curve.property_type,
                keyframe_id,
                new_time: keyframe.time,
                new_value,
            });
        }
    }
}

pub(super) fn collect_selected_key_ids(
    editor_state: &CurveEditorState,
) -> Vec<(PropertyType, KeyframeId)> {
    editor_state
        .selected_keyframes
        .iter()
        .map(|selected| (selected.property_type, selected.keyframe_id))
        .collect()
}

pub(super) fn discard_tween_on_selection_change(
    editor_state: &mut CurveEditorState,
    selection_before: &[(PropertyType, KeyframeId)],
) {
    if collect_selected_key_ids(editor_state) != selection_before {
        editor_state.tween = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::EditableKeyframe;

    fn make_curve() -> PropertyCurve {
        PropertyCurve::from_keyframes(
            0,
            PropertyType::TranslationX,
            vec![
                EditableKeyframe::new(0, 0.0, 0.0),
                EditableKeyframe::new(1, 1.0, 10.0),
                EditableKeyframe::new(2, 2.0, 20.0),
            ],
            3,
        )
    }

    #[test]
    fn curve_with_values_replaces_only_listed_keys() {
        let curve = make_curve();
        let replaced = curve_with_values(&curve, &[(1, 15.0)]);
        let values: Vec<f32> = replaced.keyframes.iter().map(|kf| kf.value).collect();
        assert_eq!(values, vec![0.0, 15.0, 20.0]);
    }

    #[test]
    fn curve_with_values_keeps_times_and_original() {
        let curve = make_curve();
        let replaced = curve_with_values(&curve, &[(0, 5.0), (2, -5.0)]);
        let times: Vec<f32> = replaced.keyframes.iter().map(|kf| kf.time).collect();
        assert_eq!(times, vec![0.0, 1.0, 2.0]);
        assert_eq!(curve.keyframes[0].value, 0.0);
        assert_eq!(replaced.keyframes[2].value, -5.0);
    }

    #[test]
    fn curve_with_values_ignores_unknown_ids() {
        let curve = make_curve();
        let replaced = curve_with_values(&curve, &[(99, 5.0)]);
        let values: Vec<f32> = replaced.keyframes.iter().map(|kf| kf.value).collect();
        assert_eq!(values, vec![0.0, 10.0, 20.0]);
    }
}
