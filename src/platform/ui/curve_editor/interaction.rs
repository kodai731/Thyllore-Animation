use imgui::MouseButton;

use super::view::*;
use super::{
    build_curve_editor_context_menu, build_keyframe_context_menu, CURVE_PADDING, TIME_RULER_HEIGHT,
    Y_AXIS_WIDTH,
};
use crate::animation::editable::{
    BezierHandle, InterpolationType, KeyframeId, PropertyCurve, PropertyType,
};
use crate::ecs::resource::{
    CurveEditorState, CurveInteractionMode, CurveSelectedKeyframe, CurveTrackRef, DraggingTangent,
    TangentHandleType, UiPointerOwnerId,
};
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::world::World;
use crate::platform::ui::pointer::{
    read_ui_pointer, ui_pointer_available, ui_pointer_begin, PointerRegion, UiPointer,
};

pub(super) const KEYFRAME_HIT_RADIUS: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SelectionModifier {
    None,
    Toggle,
    Range,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReleasedButton {
    Left,
    Middle,
}

pub(super) fn handle_curve_view_interaction(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &mut CurveEditorState,
    vt: &ViewTransform,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    cursor_pos: [f32; 2],
    curve_area_width: f32,
    clip_duration: f32,
    track_ref: CurveTrackRef,
) {
    let ruler_pos = [cursor_pos[0] + Y_AXIS_WIDTH + CURVE_PADDING, cursor_pos[1]];
    let pointer = read_ui_pointer(ui);
    let is_hovered = ui.is_item_hovered();
    let any_button_pressed = [MouseButton::Left, MouseButton::Middle, MouseButton::Right]
        .into_iter()
        .any(|button| pointer.is_clicked(button));
    let owns_press = any_button_pressed
        && is_hovered
        && ui_pointer_begin(
            ui,
            world,
            UiPointerOwnerId::CurveEditor,
            PointerRegion::LastItem,
        );
    let accepts_wheel = is_hovered
        && ui_pointer_available(
            ui,
            world,
            UiPointerOwnerId::CurveEditor,
            PointerRegion::LastItem,
        );

    handle_mouse_interaction(
        ui,
        world,
        editor_state,
        vt,
        curves_to_draw,
        &pointer,
        CurvePointerAccess {
            owns_press,
            accepts_wheel,
        },
        ruler_pos,
        curve_area_width,
        clip_duration,
    );

    if owns_press && pointer.is_clicked(MouseButton::Right) {
        let mouse_pos = pointer.pos;
        if let Some(hit) = find_keyframe_at_position(mouse_pos, curves_to_draw, vt) {
            editor_state.context_menu_keyframe = Some(CurveSelectedKeyframe {
                property_type: hit.0,
                keyframe_id: hit.1,
                original_time: hit.2,
                original_value: hit.3,
            });
            ui.open_popup("keyframe_context_menu");
        } else {
            editor_state.context_menu_click_time = vt.x_to_time(mouse_pos[0]);
            editor_state.context_menu_click_value = vt.y_to_value(mouse_pos[1]);
            ui.open_popup("curve_editor_context_menu");
        }
    }

    build_keyframe_context_menu(ui, world, editor_state, track_ref);
    build_curve_editor_context_menu(ui, world, editor_state, track_ref);
}

#[derive(Clone, Copy)]
pub(super) struct CurvePointerAccess {
    owns_press: bool,
    accepts_wheel: bool,
}

pub(super) fn handle_mouse_interaction(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &mut CurveEditorState,
    vt: &ViewTransform,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    pointer: &UiPointer,
    access: CurvePointerAccess,
    ruler_pos: [f32; 2],
    curve_area_width: f32,
    duration: f32,
) {
    let mouse_pos = pointer.pos;
    let mouse_clicked = access.owns_press && pointer.is_clicked(MouseButton::Left);
    let mouse_down = pointer.is_down(MouseButton::Left);
    let mouse_released = pointer.is_released(MouseButton::Left);
    let middle_clicked = access.owns_press && pointer.is_clicked(MouseButton::Middle);
    let middle_released = pointer.is_released(MouseButton::Middle);

    let in_ruler_area = mouse_pos[0] >= ruler_pos[0]
        && mouse_pos[0] <= ruler_pos[0] + curve_area_width
        && mouse_pos[1] >= ruler_pos[1]
        && mouse_pos[1] <= ruler_pos[1] + TIME_RULER_HEIGHT;

    let in_curve_area = mouse_pos[0] >= vt.curve_origin[0]
        && mouse_pos[0] <= vt.curve_origin[0] + vt.curve_width
        && mouse_pos[1] >= vt.curve_origin[1]
        && mouse_pos[1] <= vt.curve_origin[1] + vt.curve_height;

    if mouse_released {
        handle_mouse_release(
            world,
            editor_state,
            vt,
            mouse_pos,
            ReleasedButton::Left,
            curves_to_draw,
        );
    }
    if middle_released {
        handle_mouse_release(
            world,
            editor_state,
            vt,
            mouse_pos,
            ReleasedButton::Middle,
            curves_to_draw,
        );
    }

    if mouse_clicked && in_ruler_area {
        editor_state.interaction = CurveInteractionMode::ScrubbingRuler;
        let time = vt.x_to_time(mouse_pos[0]).clamp(0.0, duration);
        world.send_command(TimelineEvent::SetTime(time));
    }

    if mouse_clicked
        && in_curve_area
        && matches!(editor_state.interaction, CurveInteractionMode::Idle)
    {
        let modifier = if ui.io().key_ctrl {
            SelectionModifier::Toggle
        } else if ui.io().key_shift {
            SelectionModifier::Range
        } else {
            SelectionModifier::None
        };
        handle_curve_area_click(editor_state, mouse_pos, curves_to_draw, vt, modifier);
    }

    if middle_clicked && in_curve_area {
        editor_state.interaction = CurveInteractionMode::Panning {
            start_mouse_pos: mouse_pos,
            start_offset: [
                editor_state.view_time_offset,
                editor_state.view_value_offset,
            ],
        };
    }

    if matches!(
        editor_state.interaction,
        CurveInteractionMode::Panning { .. }
    ) && pointer.is_down(MouseButton::Middle)
    {
        handle_panning(editor_state, mouse_pos, vt);
    }

    if matches!(
        editor_state.interaction,
        CurveInteractionMode::ScrubbingRuler
    ) && mouse_down
    {
        let time = vt.x_to_time(mouse_pos[0]).clamp(0.0, duration);
        world.send_command(TimelineEvent::SetTime(time));
    }

    if access.accepts_wheel {
        handle_wheel_input(ui, editor_state, pointer, vt);
    }
}

pub(super) fn handle_mouse_release(
    world: &World,
    editor_state: &mut CurveEditorState,
    vt: &ViewTransform,
    mouse_pos: [f32; 2],
    button: ReleasedButton,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
) {
    match button {
        ReleasedButton::Left => {
            if let CurveInteractionMode::DraggingTangent(ref dragging) =
                editor_state.interaction.clone()
            {
                if let Some(track_ref) = editor_state.selected_track_ref() {
                    let (in_tangent, out_tangent) =
                        compute_dragged_tangent(dragging, mouse_pos, curves_to_draw, vt);
                    world.send_command(TimelineEvent::SetKeyframeTangent {
                        track: track_ref,
                        property_type: dragging.property_type,
                        keyframe_id: dragging.keyframe_id,
                        in_tangent,
                        out_tangent,
                    });
                }
            } else if matches!(
                editor_state.interaction,
                CurveInteractionMode::DraggingKeyframe
            ) {
                if let Some(track_ref) = editor_state.selected_track_ref() {
                    let time_delta = vt.x_to_time(mouse_pos[0])
                        - vt.x_to_time(editor_state.drag_start_mouse_pos[0]);
                    let value_delta = vt.y_to_value(mouse_pos[1])
                        - vt.y_to_value(editor_state.drag_start_mouse_pos[1]);

                    for sel in &editor_state.selected_keyframes {
                        world.send_command(TimelineEvent::MoveKeyframe {
                            track: track_ref,
                            property_type: sel.property_type.clone(),
                            keyframe_id: sel.keyframe_id,
                            new_time: (sel.original_time + time_delta).max(0.0),
                            new_value: sel.original_value + value_delta,
                        });
                    }
                }
            }
            editor_state.interaction = CurveInteractionMode::Idle;
        }

        ReleasedButton::Middle => {
            editor_state.interaction = CurveInteractionMode::Idle;
        }
    }
}

pub(super) fn compute_dragged_tangent(
    dragging: &DraggingTangent,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
) -> (BezierHandle, BezierHandle) {
    for (curve, _, _) in curves_to_draw {
        if curve.property_type != dragging.property_type {
            continue;
        }

        let kf = match curve.get_keyframe(dragging.keyframe_id) {
            Some(kf) => kf,
            None => break,
        };

        let mouse_time = vt.x_to_time(mouse_pos[0]);
        let mouse_value = vt.y_to_value(mouse_pos[1]);
        let time_offset = mouse_time - kf.time;
        let value_offset = mouse_value - kf.value;
        let new_handle = BezierHandle::new(time_offset, value_offset);

        return match dragging.handle_type {
            TangentHandleType::In => (new_handle, kf.out_tangent.clone()),
            TangentHandleType::Out => (kf.in_tangent.clone(), new_handle),
        };
    }

    (BezierHandle::linear(), BezierHandle::linear())
}

pub(super) fn handle_curve_area_click(
    editor_state: &mut CurveEditorState,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
    modifier: SelectionModifier,
) {
    if let Some((handle_type, property_type, keyframe_id, original_handle)) =
        find_tangent_handle_at_position(
            mouse_pos,
            curves_to_draw,
            &editor_state.selected_keyframes,
            vt,
        )
    {
        editor_state.interaction = CurveInteractionMode::DraggingTangent(DraggingTangent {
            property_type,
            keyframe_id,
            handle_type,
            original_handle,
        });
        editor_state.drag_start_mouse_pos = mouse_pos;
        return;
    }

    let hit_keyframe = find_keyframe_at_position(mouse_pos, curves_to_draw, vt);

    if let Some((property_type, keyframe_id, time, value)) = hit_keyframe {
        let new_selected = CurveSelectedKeyframe {
            property_type: property_type.clone(),
            keyframe_id,
            original_time: time,
            original_value: value,
        };

        let should_drag = match modifier {
            SelectionModifier::Toggle => {
                let existing = editor_state
                    .selected_keyframes
                    .iter()
                    .position(|s| s.keyframe_id == keyframe_id && s.property_type == property_type);

                let drag = if let Some(pos) = existing {
                    editor_state.selected_keyframes.remove(pos);
                    false
                } else {
                    editor_state.selected_keyframes.push(new_selected.clone());
                    true
                };
                editor_state.selection_anchor = Some((property_type, keyframe_id));
                drag
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
                let already_selected = editor_state
                    .selected_keyframes
                    .iter()
                    .any(|s| s.keyframe_id == keyframe_id && s.property_type == property_type);

                if !already_selected {
                    editor_state.selected_keyframes.clear();
                    editor_state.selected_keyframes.push(new_selected.clone());
                    editor_state.selection_anchor = Some((property_type, keyframe_id));
                }
                true
            }
        };

        if should_drag {
            refresh_selected_keyframe_positions(
                &mut editor_state.selected_keyframes,
                curves_to_draw,
            );
            editor_state.interaction = CurveInteractionMode::DraggingKeyframe;
            editor_state.drag_start_mouse_pos = mouse_pos;
        }
    } else if modifier == SelectionModifier::None {
        editor_state.selected_keyframes.clear();
        editor_state.selection_anchor = None;
    }
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
    let anchor = editor_state.selection_anchor.clone();

    if let Some((anchor_prop, anchor_id)) = anchor {
        if anchor_prop == *property_type {
            let anchor_time = find_keyframe_time(curves_to_draw, &anchor_prop, anchor_id);
            if let Some(anchor_time) = anchor_time {
                let range_keys =
                    collect_keyframes_in_range(curves_to_draw, property_type, anchor_time, time);
                for key in range_keys {
                    let already_exists = editor_state.selected_keyframes.iter().any(|s| {
                        s.keyframe_id == key.keyframe_id && s.property_type == key.property_type
                    });
                    if !already_exists {
                        editor_state.selected_keyframes.push(key);
                    }
                }
                return true;
            }
        }
    }

    let already_exists = editor_state
        .selected_keyframes
        .iter()
        .any(|s| s.keyframe_id == keyframe_id && s.property_type == *property_type);
    if !already_exists {
        editor_state.selected_keyframes.push(CurveSelectedKeyframe {
            property_type: property_type.clone(),
            keyframe_id,
            original_time: time,
            original_value: value,
        });
    }
    editor_state.selection_anchor = Some((property_type.clone(), keyframe_id));
    true
}

pub(super) fn find_keyframe_time(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    property_type: &PropertyType,
    keyframe_id: KeyframeId,
) -> Option<f32> {
    for (curve, _, _) in curves {
        if curve.property_type == *property_type {
            return curve.get_keyframe(keyframe_id).map(|kf| kf.time);
        }
    }
    None
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

pub(super) fn find_keyframe_at_position(
    mouse_pos: [f32; 2],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
) -> Option<(PropertyType, KeyframeId, f32, f32)> {
    for (curve, _, _) in curves {
        for kf in &curve.keyframes {
            let x = vt.time_to_x(kf.time);
            let y = vt.value_to_y(kf.value);

            let dx = mouse_pos[0] - x;
            let dy = mouse_pos[1] - y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance <= KEYFRAME_HIT_RADIUS {
                return Some((curve.property_type.clone(), kf.id, kf.time, kf.value));
            }
        }
    }

    None
}

pub(super) const TANGENT_HANDLE_HIT_RADIUS: f32 = 10.0;

pub(super) fn find_tangent_handle_at_position(
    mouse_pos: [f32; 2],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    selected_keyframes: &[CurveSelectedKeyframe],
    vt: &ViewTransform,
) -> Option<(TangentHandleType, PropertyType, KeyframeId, BezierHandle)> {
    for selected in selected_keyframes {
        for (curve, _, _) in curves {
            if curve.property_type != selected.property_type {
                continue;
            }

            let kf = match curve.get_keyframe(selected.keyframe_id) {
                Some(kf) => kf,
                None => break,
            };

            if kf.interpolation != InterpolationType::Bezier {
                break;
            }

            let in_x = vt.time_to_x(kf.time + kf.in_tangent.time_offset);
            let in_y = vt.value_to_y(kf.value + kf.in_tangent.value_offset);
            let dx = mouse_pos[0] - in_x;
            let dy = mouse_pos[1] - in_y;
            if (dx * dx + dy * dy).sqrt() <= TANGENT_HANDLE_HIT_RADIUS {
                return Some((
                    TangentHandleType::In,
                    curve.property_type,
                    kf.id,
                    kf.in_tangent.clone(),
                ));
            }

            let out_x = vt.time_to_x(kf.time + kf.out_tangent.time_offset);
            let out_y = vt.value_to_y(kf.value + kf.out_tangent.value_offset);
            let dx = mouse_pos[0] - out_x;
            let dy = mouse_pos[1] - out_y;
            if (dx * dx + dy * dy).sqrt() <= TANGENT_HANDLE_HIT_RADIUS {
                return Some((
                    TangentHandleType::Out,
                    curve.property_type,
                    kf.id,
                    kf.out_tangent.clone(),
                ));
            }

            break;
        }
    }

    None
}
