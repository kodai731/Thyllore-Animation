use imgui::MouseButton;

use super::view::*;
use super::window::{CURVE_PADDING, TIME_RULER_HEIGHT, Y_AXIS_WIDTH};
use crate::animation::editable::{
    align_opposite_handle, curve_sample, BezierHandle, InterpolationType, KeyframeId,
    PropertyCurve, PropertyType, TangentContinuity,
};
use crate::ecs::resource::{
    AxisLock, BoxSelectMode, CurveEditorState, CurveInteractionMode, CurveSelectedKeyframe,
    CurveTrackRef, DraggingTangent, TangentHandleType, TimelineState, UiPointerOwnerId,
};
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::world::World;
use crate::platform::ui::curve_editor::context_menu::{
    build_curve_editor_context_menu, build_keyframe_context_menu,
};
use crate::platform::ui::pointer::{
    read_ui_pointer, ui_pointer_available, ui_pointer_begin, PointerRegion, UiPointer,
};

pub(super) const KEYFRAME_HIT_RADIUS: f32 = 8.0;
pub(super) const CURVE_HIT_RADIUS_PX: f32 = 6.0;

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

const AXIS_LOCK_THRESHOLD_PX: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum TimeSnap {
    Frame(f32),
    Free,
}

#[must_use]
pub(super) fn resolve_axis_lock(
    current: Option<AxisLock>,
    shift_held: bool,
    pixel_delta: [f32; 2],
) -> Option<AxisLock> {
    if !shift_held {
        return None;
    }
    if current.is_some() {
        return current;
    }

    let dx = pixel_delta[0].abs();
    let dy = pixel_delta[1].abs();
    if dx.max(dy) <= AXIS_LOCK_THRESHOLD_PX {
        None
    } else if dx >= dy {
        Some(AxisLock::Time)
    } else {
        Some(AxisLock::Value)
    }
}

#[must_use]
pub(super) fn current_time_snap(ui: &imgui::Ui, frame_rate: f32) -> TimeSnap {
    if ui.io().key_ctrl {
        TimeSnap::Free
    } else {
        TimeSnap::Frame(frame_rate)
    }
}

#[must_use]
pub(super) fn dragged_key_position(
    original: [f32; 2],
    delta: [f32; 2],
    axis_lock: Option<AxisLock>,
    snap: TimeSnap,
) -> [f32; 2] {
    let locked_delta = match axis_lock {
        Some(AxisLock::Time) => [delta[0], 0.0],
        Some(AxisLock::Value) => [0.0, delta[1]],
        None => delta,
    };

    let free_time = original[0] + locked_delta[0];
    let snapped_time = match snap {
        TimeSnap::Frame(frame_rate) => (free_time * frame_rate).round() / frame_rate,
        TimeSnap::Free => free_time,
    };

    [snapped_time.max(0.0), original[1] + locked_delta[1]]
}

#[must_use]
pub(super) fn dragged_key_delta(
    curve_vt: &ViewTransform,
    drag_start: [f32; 2],
    mouse_pos: [f32; 2],
) -> [f32; 2] {
    [
        curve_vt.x_to_time(mouse_pos[0]) - curve_vt.x_to_time(drag_start[0]),
        curve_vt.y_to_value(mouse_pos[1]) - curve_vt.y_to_value(drag_start[1]),
    ]
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
            let click_vt = match find_curve_at_position(mouse_pos, curves_to_draw, vt) {
                Some(property_type) => vt.for_property(property_type, curves_to_draw),
                None => curves_to_draw
                    .first()
                    .map_or(*vt, |(curve, _, _)| vt.for_curve(curve)),
            };
            editor_state.context_menu_click_value = click_vt.y_to_value(mouse_pos[1]);
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
    let time_snap = current_time_snap(
        ui,
        world.resource::<TimelineState>().snap_settings.frame_rate,
    );

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
            time_snap,
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
            time_snap,
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
        let curve_to_insert_on =
            handle_curve_area_click(editor_state, mouse_pos, curves_to_draw, vt, modifier);
        if let (Some(property_type), Some(track)) =
            (curve_to_insert_on, editor_state.selected_track_ref())
        {
            world.send_command(TimelineEvent::InsertKeyframeOnCurve {
                track,
                property_type,
                time: vt.x_to_time(mouse_pos[0]).max(0.0),
            });
        }
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

    if let CurveInteractionMode::DraggingKeyframe { ref mut axis_lock } = editor_state.interaction {
        let pixel_delta: [f32; 2] = [
            mouse_pos[0] - editor_state.drag_start_mouse_pos[0],
            mouse_pos[1] - editor_state.drag_start_mouse_pos[1],
        ];
        *axis_lock = resolve_axis_lock(*axis_lock, ui.io().key_shift, pixel_delta);
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
    time_snap: TimeSnap,
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
            } else if let CurveInteractionMode::DraggingKeyframe { axis_lock } =
                editor_state.interaction
            {
                if let Some(track_ref) = editor_state.selected_track_ref() {
                    for sel in &editor_state.selected_keyframes {
                        let curve_vt = vt.for_property(sel.property_type, curves_to_draw);
                        let delta = dragged_key_delta(
                            &curve_vt,
                            editor_state.drag_start_mouse_pos,
                            mouse_pos,
                        );

                        let [new_time, new_value] = dragged_key_position(
                            [sel.original_time, sel.original_value],
                            delta,
                            axis_lock,
                            time_snap,
                        );
                        world.send_command(TimelineEvent::MoveKeyframe {
                            track: track_ref,
                            property_type: sel.property_type.clone(),
                            keyframe_id: sel.keyframe_id,
                            new_time,
                            new_value,
                        });
                    }
                }
            } else if let CurveInteractionMode::BoxSelecting { start, mode } =
                editor_state.interaction
            {
                let min: [f32; 2] = [start[0].min(mouse_pos[0]), start[1].min(mouse_pos[1])];
                let max: [f32; 2] = [start[0].max(mouse_pos[0]), start[1].max(mouse_pos[1])];
                let boxed = collect_keyframes_in_screen_rect(curves_to_draw, vt, min, max);
                editor_state.selected_keyframes =
                    apply_box_selection(&editor_state.selected_keyframes, boxed, mode);
                if mode == BoxSelectMode::Replace {
                    editor_state.selection_anchor = None;
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

        let curve_vt = vt.for_curve(curve);
        let mouse_time = curve_vt.x_to_time(mouse_pos[0]);
        let mouse_value = curve_vt.y_to_value(mouse_pos[1]);
        let time_offset = mouse_time - kf.time;
        let value_offset = mouse_value - kf.value;
        let new_handle = BezierHandle::new(time_offset, value_offset);

        let (in_tangent, out_tangent) = match dragging.handle_type {
            TangentHandleType::In => (new_handle, kf.out_tangent.clone()),
            TangentHandleType::Out => (kf.in_tangent.clone(), new_handle),
        };

        if kf.continuity == TangentContinuity::Unified {
            return match dragging.handle_type {
                TangentHandleType::In => {
                    let aligned_out = align_opposite_handle(&in_tangent, &out_tangent);
                    (in_tangent, aligned_out)
                }
                TangentHandleType::Out => {
                    let aligned_in = align_opposite_handle(&out_tangent, &in_tangent);
                    (aligned_in, out_tangent)
                }
            };
        }

        return (in_tangent, out_tangent);
    }

    (BezierHandle::linear(), BezierHandle::linear())
}

pub(super) fn handle_curve_area_click(
    editor_state: &mut CurveEditorState,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
    modifier: SelectionModifier,
) -> Option<PropertyType> {
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
        return None;
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
            editor_state.interaction = CurveInteractionMode::DraggingKeyframe { axis_lock: None };
            editor_state.drag_start_mouse_pos = mouse_pos;
        }
    } else {
        if modifier == SelectionModifier::Toggle {
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
    }

    None
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
        let curve_vt = vt.for_curve(curve);
        for kf in &curve.keyframes {
            let x = curve_vt.time_to_x(kf.time);
            let y = curve_vt.value_to_y(kf.value);

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

pub(super) fn find_curve_at_position(
    mouse_pos: [f32; 2],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
) -> Option<PropertyType> {
    let mouse_time = vt.x_to_time(mouse_pos[0]);

    curves
        .iter()
        .filter_map(|(curve, _, _)| {
            let value = curve_sample(curve, mouse_time)?;
            let distance = (vt.for_curve(curve).value_to_y(value) - mouse_pos[1]).abs();
            (distance <= CURVE_HIT_RADIUS_PX).then_some((curve.property_type, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(property_type, _)| property_type)
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

            let curve_vt = vt.for_curve(curve);
            let in_x = curve_vt.time_to_x(kf.time + kf.in_tangent.time_offset);
            let in_y = curve_vt.value_to_y(kf.value + kf.in_tangent.value_offset);
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

            let out_x = curve_vt.time_to_x(kf.time + kf.out_tangent.time_offset);
            let out_y = curve_vt.value_to_y(kf.value + kf.out_tangent.value_offset);
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

pub(super) fn collect_keyframes_in_screen_rect(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
    min: [f32; 2],
    max: [f32; 2],
) -> Vec<CurveSelectedKeyframe> {
    let mut result = Vec::new();
    for (curve, _, _) in curves {
        let curve_vt = vt.for_curve(curve);
        for kf in &curve.keyframes {
            let x = curve_vt.time_to_x(kf.time);
            let y = curve_vt.value_to_y(kf.value);
            if x >= min[0] && x <= max[0] && y >= min[1] && y <= max[1] {
                result.push(CurveSelectedKeyframe {
                    property_type: curve.property_type.clone(),
                    keyframe_id: kf.id,
                    original_time: kf.time,
                    original_value: kf.value,
                });
            }
        }
    }
    result
}

pub(super) fn apply_box_selection(
    current: &[CurveSelectedKeyframe],
    boxed: Vec<CurveSelectedKeyframe>,
    mode: BoxSelectMode,
) -> Vec<CurveSelectedKeyframe> {
    match mode {
        BoxSelectMode::Replace => boxed,
        BoxSelectMode::Add => {
            let mut result = current.to_vec();
            for key in boxed {
                let already_exists = result.iter().any(|s| {
                    s.keyframe_id == key.keyframe_id && s.property_type == key.property_type
                });
                if !already_exists {
                    result.push(key);
                }
            }
            result
        }
        BoxSelectMode::Invert => {
            let mut result = current.to_vec();
            for key in boxed {
                let pos = result.iter().position(|s| {
                    s.keyframe_id == key.keyframe_id && s.property_type == key.property_type
                });
                if let Some(p) = pos {
                    result.remove(p);
                } else {
                    result.push(key);
                }
            }
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::curve_add_keyframe;
    use crate::ecs::resource::CurveValueDisplay;

    fn build_unit_view() -> ViewTransform {
        ViewTransform {
            curve_origin: [0.0, 0.0],
            curve_width: 1.0,
            curve_height: 1.0,
            duration: 1.0,
            val_range: 1.0,
            zoom_x: 1.0,
            zoom_y: 1.0,
            view_time_offset: 0.0,
            view_value_offset: 0.0,
            value_display: CurveValueDisplay::Actual,
        }
    }

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

    fn build_pixel_view() -> ViewTransform {
        ViewTransform {
            curve_width: 100.0,
            curve_height: 100.0,
            ..build_unit_view()
        }
    }

    fn build_flat_curve(property_type: PropertyType, value: f32) -> PropertyCurve {
        let mut curve = PropertyCurve::new(0, property_type);
        curve_add_keyframe(&mut curve, 0.0, value);
        curve_add_keyframe(&mut curve, 1.0, value);
        curve
    }

    #[test]
    fn find_curve_at_position_picks_the_nearest_curve_within_the_radius() {
        let lower = build_flat_curve(PropertyType::TranslationX, 0.2);
        let upper = build_flat_curve(PropertyType::TranslationY, 0.25);
        let curves = [(&lower, [1.0; 4], "x"), (&upper, [1.0; 4], "y")];

        let hit = find_curve_at_position([50.0, 78.0], &curves, &build_pixel_view());

        assert_eq!(hit, Some(PropertyType::TranslationX));
    }

    #[test]
    fn find_curve_at_position_misses_beyond_the_radius() {
        let curve = build_flat_curve(PropertyType::TranslationX, 0.2);
        let curves = [(&curve, [1.0; 4], "x")];

        let hit = find_curve_at_position(
            [50.0, 80.0 - CURVE_HIT_RADIUS_PX - 1.0],
            &curves,
            &build_pixel_view(),
        );

        assert_eq!(hit, None);
    }

    fn build_normalized_pixel_view() -> ViewTransform {
        ViewTransform {
            val_range: 2.0,
            view_value_offset: -1.0,
            value_display: CurveValueDisplay::Normalized,
            ..build_pixel_view()
        }
    }

    fn build_two_point_curve(property_type: PropertyType, values: [f32; 2]) -> PropertyCurve {
        let mut curve = PropertyCurve::new(0, property_type);
        curve_add_keyframe(&mut curve, 0.0, values[0]);
        curve_add_keyframe(&mut curve, 1.0, values[1]);
        curve
    }

    #[test]
    fn normalized_keyframe_hit_uses_each_curve_range() {
        let small = build_two_point_curve(PropertyType::TranslationX, [0.0, 10.0]);
        let large = build_two_point_curve(PropertyType::TranslationY, [100.0, -100.0]);
        let curves = [(&small, [1.0; 4], "x"), (&large, [1.0; 4], "y")];
        let vt = build_normalized_pixel_view();

        let bottom_hit = find_keyframe_at_position([0.0, 100.0], &curves, &vt);
        let top_hit = find_keyframe_at_position([0.0, 0.0], &curves, &vt);

        assert_eq!(
            bottom_hit.map(|hit| (hit.0, hit.3)),
            Some((PropertyType::TranslationX, 0.0))
        );
        assert_eq!(
            top_hit.map(|hit| (hit.0, hit.3)),
            Some((PropertyType::TranslationY, 100.0))
        );
    }

    #[test]
    fn normalized_drag_value_delta_scales_with_each_curve_half_width() {
        let small = build_two_point_curve(PropertyType::TranslationX, [0.0, 10.0]);
        let large = build_two_point_curve(PropertyType::TranslationY, [100.0, -100.0]);
        let curves = [(&small, [1.0; 4], "x"), (&large, [1.0; 4], "y")];
        let vt = build_normalized_pixel_view();
        let drag_start = [50.0, 50.0];
        let mouse_pos = [50.0, 40.0];
        let normalized_delta = 0.2;

        let small_delta = dragged_key_delta(
            &vt.for_property(PropertyType::TranslationX, &curves),
            drag_start,
            mouse_pos,
        );
        let large_delta = dragged_key_delta(
            &vt.for_property(PropertyType::TranslationY, &curves),
            drag_start,
            mouse_pos,
        );

        assert!((small_delta[1] - normalized_delta * 5.0).abs() < 1e-4);
        assert!((large_delta[1] - normalized_delta * 100.0).abs() < 1e-3);
        assert_eq!(small_delta[0], large_delta[0]);
    }

    #[test]
    fn axis_lock_stays_unresolved_below_the_threshold() {
        assert_eq!(resolve_axis_lock(None, true, [3.0, -3.0]), None);
    }

    #[test]
    fn axis_lock_picks_time_for_a_horizontal_move() {
        assert_eq!(
            resolve_axis_lock(None, true, [6.0, 1.0]),
            Some(AxisLock::Time)
        );
    }

    #[test]
    fn axis_lock_picks_value_for_a_vertical_move() {
        assert_eq!(
            resolve_axis_lock(None, true, [1.0, -6.0]),
            Some(AxisLock::Value)
        );
    }

    #[test]
    fn axis_lock_keeps_the_first_direction_while_shift_is_held() {
        assert_eq!(
            resolve_axis_lock(Some(AxisLock::Time), true, [6.0, 40.0]),
            Some(AxisLock::Time)
        );
    }

    #[test]
    fn axis_lock_is_released_with_shift() {
        assert_eq!(
            resolve_axis_lock(Some(AxisLock::Time), false, [6.0, 0.0]),
            None
        );
    }

    #[test]
    fn dragged_key_position_time_lock_keeps_the_value() {
        let position =
            dragged_key_position([1.0, 2.0], [0.5, 3.0], Some(AxisLock::Time), TimeSnap::Free);

        assert_eq!(position, [1.5, 2.0]);
    }

    #[test]
    fn dragged_key_position_value_lock_keeps_the_time() {
        let position = dragged_key_position(
            [1.0, 2.0],
            [0.5, 3.0],
            Some(AxisLock::Value),
            TimeSnap::Free,
        );

        assert_eq!(position, [1.0, 5.0]);
    }

    #[test]
    fn dragged_key_position_snaps_time_to_30fps_frames() {
        let [time, value] =
            dragged_key_position([0.0, 1.0], [0.05, 0.25], None, TimeSnap::Frame(30.0));

        assert!((time - 2.0 / 30.0).abs() < 1e-6);
        assert_eq!(value, 1.25);
    }

    #[test]
    fn dragged_key_position_free_keeps_the_raw_time_and_clamps_at_zero() {
        assert_eq!(
            dragged_key_position([1.0, 0.0], [0.013, 0.0], None, TimeSnap::Free),
            [1.013, 0.0]
        );
        assert_eq!(
            dragged_key_position([0.1, 0.0], [-0.5, 0.0], None, TimeSnap::Frame(30.0)),
            [0.0, 0.0]
        );
    }
}
