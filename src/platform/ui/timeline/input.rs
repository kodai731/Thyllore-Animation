use super::*;

pub(super) fn handle_timeline_shortcuts(ui: &imgui::Ui, world: &World, state: &TimelineState) {
    let io = ui.io();
    let focused = ui.is_window_focused_with_flags(imgui::FocusedFlags::ROOT_AND_CHILD_WINDOWS);
    if !focused || ui.is_any_item_active() {
        return;
    }

    if io.key_ctrl() && ui.is_key_pressed(imgui::Key::C) {
        world.send_command(TimelineEvent::CopyKeyframes);
    }

    if io.key_ctrl() && !io.key_shift() && ui.is_key_pressed(imgui::Key::V) {
        world.send_command(TimelineEvent::PasteKeyframes {
            paste_time: state.current_time,
        });
    }

    if io.key_ctrl() && io.key_shift() && ui.is_key_pressed(imgui::Key::V) {
        world.send_command(TimelineEvent::MirrorPaste {
            paste_time: state.current_time,
        });
    }

    if ui.is_key_pressed(imgui::Key::Delete) {
        delete_timeline_selection(world, state);
    }
}

pub(super) fn delete_timeline_selection(world: &World, state: &TimelineState) {
    if !state.selected_keyframes.is_empty() {
        world.send_command(TimelineEvent::DeleteSelectedKeyframes);
        return;
    }

    if let Some((entity, instance_id)) = state.selected_clip_instance {
        world.send_command(ClipInstanceEvent::Delete {
            entity,
            instance_id,
        });
    }
}

pub(super) fn compute_zoom_limits(
    available_width: f32,
    clip_duration: f32,
    frame_rate: f32,
) -> (f32, f32) {
    let width = available_width.max(1.0);
    let fps = frame_rate.max(1.0);
    let duration = clip_duration.max(0.1);

    let three_frames_duration = 3.0 / fps;
    let max_zoom = width / (three_frames_duration * PIXELS_PER_SECOND);

    let four_clips_duration = duration * 4.0;
    let min_zoom = width / (four_clips_duration * PIXELS_PER_SECOND);

    (min_zoom.max(0.01), max_zoom)
}

pub(super) fn handle_mouse_wheel_zoom(
    ui: &imgui::Ui,
    world: &World,
    state: &TimelineState,
    clip_duration: f32,
) {
    let available = ui_pointer_available(
        ui,
        world,
        UiPointerOwnerId::Timeline,
        PointerRegion::WindowWithChildren,
    );
    if !available {
        return;
    }

    let available_width = state.last_visible_width.max(1.0);
    let (min_zoom, max_zoom) = compute_zoom_limits(
        available_width,
        clip_duration,
        state.snap_settings.frame_rate,
    );

    let wheel = read_ui_pointer(ui).wheel;
    if wheel > 0.0 {
        world.send_command(TimelineEvent::ZoomIn { max_zoom });
    } else if wheel < 0.0 {
        world.send_command(TimelineEvent::ZoomOut { min_zoom });
    }
}

pub(super) fn handle_middle_drag_pan(ui: &imgui::Ui, world: &World, state: &mut TimelineState) {
    let pointer = read_ui_pointer(ui);
    if !pointer.is_down(MouseButton::Middle) {
        return;
    }

    let owns_pointer = ui_pointer_begin(
        ui,
        world,
        UiPointerOwnerId::Timeline,
        PointerRegion::WindowWithChildren,
    );
    if !owns_pointer {
        return;
    }

    let delta_x = pointer.delta[0];
    if delta_x.abs() < 0.01 {
        return;
    }

    state.pan_pending_delta_x -= delta_x;
}
