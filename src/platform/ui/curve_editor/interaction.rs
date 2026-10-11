use imgui::MouseButton;

use super::drag::*;
use super::hit_test::*;
use super::selection::*;
use super::view::*;
use super::window::{CURVE_PADDING, TIME_RULER_HEIGHT, Y_AXIS_WIDTH};
use crate::animation::editable::PropertyCurve;
use crate::ecs::resource::{
    AxisLock, BoxSelectMode, CurveEditorState, CurveInteractionMode, CurveSelectedKeyframe,
    CurveTrackRef, CurveViewMode, DraggingTangent, TimelineState, UiPointerOwnerId,
};
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::world::World;
use crate::platform::ui::curve_editor::context_menu::{
    build_curve_editor_context_menu, build_keyframe_context_menu,
};
use crate::platform::ui::pointer::{
    read_ui_pointer, ui_pointer_available, ui_pointer_begin, PointerRegion, UiPointer,
};

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
        let modifier = if ui.io().key_ctrl() {
            SelectionModifier::Toggle
        } else if ui.io().key_shift() {
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
        *axis_lock = resolve_axis_lock(*axis_lock, ui.io().key_shift(), pixel_delta);
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
    if button == ReleasedButton::Left {
        finish_left_drag(
            world,
            editor_state,
            vt,
            mouse_pos,
            curves_to_draw,
            time_snap,
        );
    }
    editor_state.interaction = CurveInteractionMode::Idle;
}

fn finish_left_drag(
    world: &World,
    editor_state: &mut CurveEditorState,
    vt: &ViewTransform,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    time_snap: TimeSnap,
) {
    match editor_state.interaction.clone() {
        CurveInteractionMode::DraggingTangent(dragging) => {
            finish_tangent_drag(world, editor_state, vt, mouse_pos, curves_to_draw, dragging);
        }
        CurveInteractionMode::DraggingKeyframe { axis_lock } => {
            finish_keyframe_drag(
                world,
                editor_state,
                vt,
                mouse_pos,
                curves_to_draw,
                time_snap,
                axis_lock,
            );
        }
        CurveInteractionMode::BoxSelecting { start, mode } => {
            finish_box_selection(editor_state, vt, mouse_pos, curves_to_draw, start, mode);
        }
        CurveInteractionMode::Idle
        | CurveInteractionMode::ScrubbingRuler
        | CurveInteractionMode::Panning { .. } => {}
    }
}

fn finish_tangent_drag(
    world: &World,
    editor_state: &CurveEditorState,
    vt: &ViewTransform,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    dragging: DraggingTangent,
) {
    let Some(track_ref) = editor_state.selected_track_ref() else {
        return;
    };
    let (in_tangent, out_tangent) =
        compute_dragged_tangent(&dragging, mouse_pos, curves_to_draw, vt);
    world.send_command(TimelineEvent::SetKeyframeTangent {
        track: track_ref,
        property_type: dragging.property_type,
        keyframe_id: dragging.keyframe_id,
        in_tangent,
        out_tangent,
    });
}

fn finish_keyframe_drag(
    world: &World,
    editor_state: &CurveEditorState,
    vt: &ViewTransform,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    time_snap: TimeSnap,
    axis_lock: Option<AxisLock>,
) {
    let Some(track_ref) = editor_state.selected_track_ref() else {
        return;
    };
    for sel in &editor_state.selected_keyframes {
        let curve_vt = vt.for_property(sel.property_type, curves_to_draw);
        let delta = dragged_key_delta(&curve_vt, editor_state.drag_start_mouse_pos, mouse_pos);

        let [new_time, new_value] = dragged_key_position(
            [sel.original_time, sel.original_value],
            delta,
            effective_axis_lock(editor_state.view_mode, axis_lock),
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

fn finish_box_selection(
    editor_state: &mut CurveEditorState,
    vt: &ViewTransform,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    start: [f32; 2],
    mode: BoxSelectMode,
) {
    let min: [f32; 2] = [start[0].min(mouse_pos[0]), start[1].min(mouse_pos[1])];
    let max: [f32; 2] = [start[0].max(mouse_pos[0]), start[1].max(mouse_pos[1])];
    let boxed = match editor_state.view_mode {
        CurveViewMode::Curves => collect_keyframes_in_screen_rect(curves_to_draw, vt, min, max),
        CurveViewMode::Dopesheet => {
            super::dopesheet::collect_dopesheet_keys_in_rect(curves_to_draw, vt, min, max)
        }
    };
    editor_state.selected_keyframes =
        apply_box_selection(&editor_state.selected_keyframes, boxed, mode);
    if mode == BoxSelectMode::Replace {
        editor_state.selection_anchor = None;
    }
}
