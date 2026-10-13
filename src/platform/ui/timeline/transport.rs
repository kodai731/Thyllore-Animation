use super::*;

pub(super) fn build_transport_controls(
    ui: &imgui::Ui,
    world: &World,
    state: &mut TimelineState,
    clip_library: &ClipLibrary,
    curve_editor_state: &mut CurveEditorState,
) {
    if state.playing {
        if icon_button(ui, Icon::Pause, "Pause", ButtonState::Active) {
            world.send_command(TimelineEvent::Pause);
        }
    } else if icon_button(ui, Icon::Play, "Play", ButtonState::Normal) {
        world.send_command(TimelineEvent::Play);
    }
    ui.same_line_with_spacing(0.0, 4.0);
    if icon_button(ui, Icon::Stop, "Stop", ButtonState::Normal) {
        world.send_command(TimelineEvent::Stop);
    }

    toolbar_divider(ui);

    let loop_state = if state.looping {
        ButtonState::Active
    } else {
        ButtonState::Normal
    };
    if icon_button(ui, Icon::Repeat, "Loop", loop_state) {
        world.send_command(TimelineEvent::ToggleLoop);
    }

    toolbar_divider(ui);

    let preview_options = [ClipPreview::Solo, ClipPreview::Mix];
    let preview_selected = match state.preview {
        ClipPreview::Solo => 0,
        ClipPreview::Mix => 1,
    };
    if let Some(index) = segmented_control(ui, "preview", &["Solo", "Mix"], preview_selected) {
        world.send_command(TimelineEvent::SetPreview(preview_options[index]));
    }

    toolbar_divider(ui);

    let current_clip = state.current_clip_id.and_then(|id| clip_library.get(id));
    let duration = timeline_effective_duration(state, clip_library);

    ui.text(format!(
        "Time: {:.2}s / {:.2}s",
        state.current_time, duration
    ));

    let available_width = state.last_visible_width.max(1.0);
    let (min_zoom, max_zoom) = compute_zoom_limits(
        available_width,
        duration.max(1.0),
        state.snap_settings.frame_rate,
    );

    ui.same_line_with_spacing(0.0, 4.0);
    if icon_button(ui, Icon::Minus, "Zoom Out", ButtonState::Normal) {
        world.send_command(TimelineEvent::ZoomOut { min_zoom });
    }
    ui.same_line_with_spacing(0.0, 4.0);
    if icon_button(ui, Icon::Plus, "Zoom In", ButtonState::Normal) {
        world.send_command(TimelineEvent::ZoomIn { max_zoom });
    }
    ui.same_line_with_spacing(0.0, 4.0);
    ui.text(format!("Zoom: {:.1}x", state.zoom_level));

    toolbar_divider(ui);

    if icon_button(ui, Icon::Spline, "Curves", ButtonState::Normal) {
        curve_editor_state.open_for_clip(current_clip);
    }

    build_clip_selector(ui, world, state, clip_library);

    build_snap_controls(ui, world, state);
}

pub(super) fn build_clip_selector(
    ui: &imgui::Ui,
    world: &World,
    state: &TimelineState,
    clip_library: &ClipLibrary,
) {
    let clip_names =
        crate::ecs::systems::clip_library_systems::clip_library_clip_names(clip_library);

    if clip_names.is_empty() {
        ui.text("No clips available");
        return;
    }

    let current_display = state
        .current_clip_id
        .and_then(|id| clip_library.get(id))
        .map(|c| build_clip_display_name(&c.name, c.source_path.as_deref()))
        .unwrap_or_else(|| "Select Clip".to_string());

    ui.same_line();
    ui.set_next_item_width(200.0);

    if let Some(_token) = ui.begin_combo("##clip_select", &current_display) {
        for (id, name) in &clip_names {
            let is_selected = state.current_clip_id == Some(*id);
            let source_path = clip_library.get(*id).and_then(|c| c.source_path.clone());
            let display = build_clip_display_name(name, source_path.as_deref());
            let label = format!("{}##clip_select_{}", display, id);
            if ui.selectable_config(&label).selected(is_selected).build() {
                world.send_command(TimelineEvent::SelectClip(*id));
            }
        }
    }
}

pub(super) fn build_snap_controls(ui: &imgui::Ui, world: &World, state: &TimelineState) {
    let snap = &state.snap_settings;

    let frame_label = if snap.snap_to_frame {
        "[Snap: F]"
    } else {
        "Snap: F"
    };

    if ui.small_button(frame_label) {
        world.send_command(TimelineEvent::SetSnapToFrame(!snap.snap_to_frame));
    }

    ui.same_line();

    let key_label = if snap.snap_to_key {
        "[Snap: K]"
    } else {
        "Snap: K"
    };

    if ui.small_button(key_label) {
        world.send_command(TimelineEvent::SetSnapToKey(!snap.snap_to_key));
    }

    ui.same_line();

    let fps_options = [24.0_f32, 30.0, 60.0];
    let current_fps_label = format!("{}fps", snap.frame_rate as u32);
    ui.set_next_item_width(70.0);

    if let Some(_token) = ui.begin_combo("##fps_select", &current_fps_label) {
        for fps in &fps_options {
            let label = format!("{}fps", *fps as u32);
            let is_selected = (snap.frame_rate - fps).abs() < 0.1;
            if ui.selectable_config(&label).selected(is_selected).build() {
                world.send_command(TimelineEvent::SetFrameRate(*fps));
            }
        }
    }
}
