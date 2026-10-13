use super::hit_test::*;
use super::interaction::SelectionModifier;
use super::view::*;
use crate::animation::editable::{KeyframeId, PropertyCurve, PropertyType};
use crate::ecs::resource::{
    BoxSelectMode, CurveEditorState, CurveInteractionMode, CurveSelectedKeyframe, CurveViewMode,
    DraggingTangent,
};

pub(super) fn handle_curve_area_click(
    editor_state: &mut CurveEditorState,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
    modifier: SelectionModifier,
) -> Option<PropertyType> {
    if try_hit_tangent_handle(editor_state, mouse_pos, curves_to_draw, vt) {
        return None;
    }

    let hit_keyframe = match editor_state.view_mode {
        CurveViewMode::Curves => find_keyframe_at_position(mouse_pos, curves_to_draw, vt),
        CurveViewMode::Dopesheet => {
            super::dopesheet::find_dopesheet_key(mouse_pos, curves_to_draw, vt).map(|key| {
                (
                    key.property_type,
                    key.keyframe_id,
                    key.original_time,
                    key.original_value,
                )
            })
        }
    };

    let Some((property_type, keyframe_id, time, value)) = hit_keyframe else {
        return handle_curve_area_miss(editor_state, mouse_pos, curves_to_draw, vt, modifier);
    };

    let new_selected = CurveSelectedKeyframe {
        property_type: property_type.clone(),
        keyframe_id,
        original_time: time,
        original_value: value,
    };

    let should_drag = match modifier {
        SelectionModifier::Toggle => {
            handle_toggle_selection(editor_state, &property_type, keyframe_id, new_selected)
        }
        SelectionModifier::Range => apply_shift_range_selection(
            editor_state,
            curves_to_draw,
            &property_type,
            keyframe_id,
            time,
            value,
        ),
        SelectionModifier::None => {
            handle_replace_selection(editor_state, &property_type, keyframe_id, new_selected)
        }
    };

    if should_drag {
        refresh_selected_keyframe_positions(&mut editor_state.selected_keyframes, curves_to_draw);
        editor_state.interaction = CurveInteractionMode::DraggingKeyframe { axis_lock: None };
        editor_state.drag_start_mouse_pos = mouse_pos;
    }

    None
}

fn try_hit_tangent_handle(
    editor_state: &mut CurveEditorState,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
) -> bool {
    if editor_state.view_mode != CurveViewMode::Curves {
        return false;
    }
    let Some((handle_type, property_type, keyframe_id, original_handle)) =
        find_tangent_handle_at_position(
            mouse_pos,
            curves_to_draw,
            &editor_state.selected_keyframes,
            vt,
        )
    else {
        return false;
    };

    editor_state.interaction = CurveInteractionMode::DraggingTangent(DraggingTangent {
        property_type,
        keyframe_id,
        handle_type,
        original_handle,
    });
    editor_state.drag_start_mouse_pos = mouse_pos;
    true
}

fn handle_curve_area_miss(
    editor_state: &mut CurveEditorState,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
    modifier: SelectionModifier,
) -> Option<PropertyType> {
    if modifier == SelectionModifier::Toggle && editor_state.view_mode == CurveViewMode::Curves {
        if let Some(property_type) = find_curve_at_position(mouse_pos, curves_to_draw, vt) {
            return Some(property_type);
        }
    }

    let mode = match modifier {
        SelectionModifier::Toggle => BoxSelectMode::Invert,
        SelectionModifier::Range => BoxSelectMode::Add,
        SelectionModifier::None => BoxSelectMode::Replace,
    };
    editor_state.interaction = CurveInteractionMode::BoxSelecting {
        start: mouse_pos,
        mode,
    };
    None
}

fn handle_toggle_selection(
    editor_state: &mut CurveEditorState,
    property_type: &PropertyType,
    keyframe_id: KeyframeId,
    new_selected: CurveSelectedKeyframe,
) -> bool {
    let existing = editor_state
        .selected_keyframes
        .iter()
        .position(|s| s.keyframe_id == keyframe_id && s.property_type == *property_type);

    let drag = if let Some(pos) = existing {
        editor_state.selected_keyframes.remove(pos);
        false
    } else {
        editor_state.selected_keyframes.push(new_selected);
        true
    };
    editor_state.selection_anchor = Some((*property_type, keyframe_id));
    drag
}

fn handle_replace_selection(
    editor_state: &mut CurveEditorState,
    property_type: &PropertyType,
    keyframe_id: KeyframeId,
    new_selected: CurveSelectedKeyframe,
) -> bool {
    let already_selected = editor_state
        .selected_keyframes
        .iter()
        .any(|s| s.keyframe_id == keyframe_id && s.property_type == *property_type);

    if !already_selected {
        editor_state.selected_keyframes.clear();
        editor_state.selected_keyframes.push(new_selected);
        editor_state.selection_anchor = Some((*property_type, keyframe_id));
    }
    true
}

pub(super) fn refresh_selected_keyframe_positions(
    selected: &mut [CurveSelectedKeyframe],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
) {
    for sel in selected.iter_mut() {
        for (curve, _, _) in curves {
            if curve.property_type != sel.property_type {
                continue;
            }
            if let Some(kf) = curve.get_keyframe(sel.keyframe_id) {
                sel.original_time = kf.time;
                sel.original_value = kf.value;
            }
            break;
        }
    }
}

pub(super) fn apply_shift_range_selection(
    editor_state: &mut CurveEditorState,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    property_type: &PropertyType,
    keyframe_id: KeyframeId,
    time: f32,
    value: f32,
) -> bool {
    let anchor_time = editor_state
        .selection_anchor
        .filter(|(anchor_prop, _)| anchor_prop == property_type)
        .and_then(|(anchor_prop, anchor_id)| {
            find_keyframe_time(curves_to_draw, &anchor_prop, anchor_id)
        });

    if let Some(anchor_time) = anchor_time {
        let range_keys =
            collect_keyframes_in_range(curves_to_draw, property_type, anchor_time, time);
        for key in range_keys {
            push_if_unselected(&mut editor_state.selected_keyframes, key);
        }
        return true;
    }

    push_if_unselected(
        &mut editor_state.selected_keyframes,
        CurveSelectedKeyframe {
            property_type: property_type.clone(),
            keyframe_id,
            original_time: time,
            original_value: value,
        },
    );
    editor_state.selection_anchor = Some((property_type.clone(), keyframe_id));
    true
}

fn push_if_unselected(selected: &mut Vec<CurveSelectedKeyframe>, key: CurveSelectedKeyframe) {
    let already_selected = selected
        .iter()
        .any(|s| s.keyframe_id == key.keyframe_id && s.property_type == key.property_type);
    if !already_selected {
        selected.push(key);
    }
}

pub(super) fn collect_keyframes_in_range(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    property_type: &PropertyType,
    time_a: f32,
    time_b: f32,
) -> Vec<CurveSelectedKeyframe> {
    let min_time = time_a.min(time_b);
    let max_time = time_a.max(time_b);

    for (curve, _, _) in curves {
        if curve.property_type == *property_type {
            return curve
                .keyframes
                .iter()
                .filter(|kf| kf.time >= min_time && kf.time <= max_time)
                .map(|kf| CurveSelectedKeyframe {
                    property_type: property_type.clone(),
                    keyframe_id: kf.id,
                    original_time: kf.time,
                    original_value: kf.value,
                })
                .collect();
        }
    }
    Vec::new()
}

pub(super) fn collect_keyframes_in_screen_rect(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
    min: [f32; 2],
    max: [f32; 2],
) -> Vec<CurveSelectedKeyframe> {
    curves
        .iter()
        .flat_map(|(curve, _, _)| {
            let curve_vt = vt.for_curve(curve);
            curve.keyframes.iter().filter_map(move |kf| {
                let x = curve_vt.time_to_x(kf.time);
                let y = curve_vt.value_to_y(kf.value);
                let inside = x >= min[0] && x <= max[0] && y >= min[1] && y <= max[1];
                inside.then(|| CurveSelectedKeyframe {
                    property_type: curve.property_type.clone(),
                    keyframe_id: kf.id,
                    original_time: kf.time,
                    original_value: kf.value,
                })
            })
        })
        .collect()
}

pub(super) fn apply_box_selection(
    current: &[CurveSelectedKeyframe],
    boxed: Vec<CurveSelectedKeyframe>,
    mode: BoxSelectMode,
) -> Vec<CurveSelectedKeyframe> {
    if mode == BoxSelectMode::Replace {
        return boxed;
    }

    let mut result = current.to_vec();
    for key in boxed {
        let existing = result
            .iter()
            .position(|s| s.keyframe_id == key.keyframe_id && s.property_type == key.property_type);
        match (mode, existing) {
            (BoxSelectMode::Invert, Some(pos)) => {
                result.remove(pos);
            }
            (BoxSelectMode::Add, Some(_)) | (BoxSelectMode::Replace, _) => {}
            (BoxSelectMode::Add | BoxSelectMode::Invert, None) => result.push(key),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::curve_add_keyframe;

    use super::super::test_fixtures::*;

    fn build_curve_with_keys(keys: &[(f32, f32)]) -> (PropertyCurve, Vec<KeyframeId>) {
        let mut curve = PropertyCurve::new(0, PropertyType::TranslationX);
        let ids = keys
            .iter()
            .map(|&(time, value)| curve_add_keyframe(&mut curve, time, value))
            .collect();
        (curve, ids)
    }

    fn build_selection(curve: &PropertyCurve, ids: &[KeyframeId]) -> Vec<CurveSelectedKeyframe> {
        ids.iter()
            .map(|&keyframe_id| CurveSelectedKeyframe {
                property_type: curve.property_type,
                keyframe_id,
                original_time: 0.0,
                original_value: 0.0,
            })
            .collect()
    }

    fn collect_selected_ids(selection: &[CurveSelectedKeyframe]) -> Vec<KeyframeId> {
        let mut ids: Vec<KeyframeId> = selection.iter().map(|key| key.keyframe_id).collect();
        ids.sort();
        ids
    }

    #[test]
    fn box_selection_collects_only_keys_inside_the_rect() {
        let (curve, ids) = build_curve_with_keys(&[(0.2, 0.2), (0.4, 0.4), (0.8, 0.8)]);
        let curves = [(&curve, [1.0; 4], "x")];

        let boxed =
            collect_keyframes_in_screen_rect(&curves, &build_unit_view(), [0.1, 0.5], [0.5, 0.9]);

        assert_eq!(collect_selected_ids(&boxed), vec![ids[0], ids[1]]);
    }

    #[test]
    fn box_selection_replace_drops_the_previous_selection() {
        let (curve, ids) = build_curve_with_keys(&[(0.2, 0.2), (0.4, 0.4), (0.8, 0.8)]);
        let current = build_selection(&curve, &[ids[2]]);
        let boxed = build_selection(&curve, &[ids[0], ids[1]]);

        let selection = apply_box_selection(&current, boxed, BoxSelectMode::Replace);

        assert_eq!(collect_selected_ids(&selection), vec![ids[0], ids[1]]);
    }

    #[test]
    fn box_selection_replace_with_an_empty_rect_clears() {
        let (curve, ids) = build_curve_with_keys(&[(0.2, 0.2)]);
        let current = build_selection(&curve, &[ids[0]]);

        let selection = apply_box_selection(&current, Vec::new(), BoxSelectMode::Replace);

        assert!(selection.is_empty());
    }

    #[test]
    fn box_selection_add_keeps_the_previous_selection_without_duplicates() {
        let (curve, ids) = build_curve_with_keys(&[(0.2, 0.2), (0.4, 0.4), (0.8, 0.8)]);
        let current = build_selection(&curve, &[ids[1], ids[2]]);
        let boxed = build_selection(&curve, &[ids[0], ids[1]]);

        let selection = apply_box_selection(&current, boxed, BoxSelectMode::Add);

        assert_eq!(
            collect_selected_ids(&selection),
            vec![ids[0], ids[1], ids[2]]
        );
    }

    #[test]
    fn box_selection_invert_toggles_only_the_boxed_keys() {
        let (curve, ids) = build_curve_with_keys(&[(0.2, 0.2), (0.4, 0.4), (0.8, 0.8)]);
        let current = build_selection(&curve, &[ids[1], ids[2]]);
        let boxed = build_selection(&curve, &[ids[0], ids[1]]);

        let selection = apply_box_selection(&current, boxed, BoxSelectMode::Invert);

        assert_eq!(collect_selected_ids(&selection), vec![ids[0], ids[2]]);
    }
}
