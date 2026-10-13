use super::*;

pub(super) fn draw_timeline_window(
    ui: &imgui::Ui,
    world: &World,
    state: &mut TimelineState,
    interaction: &mut TimelineInteractionState,
    clip_library: &ClipLibrary,
    curve_editor_state: &mut CurveEditorState,
    clip_track_snapshot: &ClipTrackSnapshot,
    layout: &LayoutSnapshot,
    curve_buffer: &CurveEditorBuffer,
    pose_library: &mut PoseLibrary,
) {
    ui.window("Timeline")
        .position([0.0, layout.timeline_y], Condition::Always)
        .size(
            [layout.display_size[0], layout.timeline_height],
            Condition::Always,
        )
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .bring_to_front_on_focus(false)
        .build(|| {
            build_transport_controls(ui, world, state, clip_library, curve_editor_state);
            ui.separator();
            draw_timeline_tabs(
                ui,
                world,
                state,
                interaction,
                clip_library,
                curve_editor_state,
                clip_track_snapshot,
                curve_buffer,
                pose_library,
            );
        });
}

pub(super) fn draw_timeline_tabs(
    ui: &imgui::Ui,
    world: &World,
    state: &mut TimelineState,
    interaction: &mut TimelineInteractionState,
    clip_library: &ClipLibrary,
    curve_editor_state: &mut CurveEditorState,
    clip_track_snapshot: &ClipTrackSnapshot,
    curve_buffer: &CurveEditorBuffer,
    pose_library: &mut PoseLibrary,
) {
    let Some(_tab_bar) = ui.tab_bar("timeline_tabs") else {
        return;
    };
    let curves_flags = if curve_editor_state.needs_focus {
        imgui::TabItemFlags::SET_SELECTED
    } else {
        imgui::TabItemFlags::empty()
    };
    curve_editor_state.needs_focus = false;

    if let Some(_tab) = ui.tab_item("Timeline") {
        curve_editor_state.is_open = false;
        draw_timeline_tab(
            ui,
            world,
            state,
            interaction,
            clip_library,
            curve_editor_state,
            clip_track_snapshot,
        );
    }
    if let Some(_tab) = ui.tab_item_with_flags("Curves", None, curves_flags) {
        curve_editor_state.is_open = true;
        draw_curves_tab(
            ui,
            world,
            state,
            clip_library,
            curve_editor_state,
            curve_buffer,
            pose_library,
        );
    }
}

pub(super) fn draw_curves_tab(
    ui: &imgui::Ui,
    world: &World,
    state: &TimelineState,
    clip_library: &ClipLibrary,
    curve_editor_state: &mut CurveEditorState,
    curve_buffer: &CurveEditorBuffer,
    pose_library: &mut PoseLibrary,
) {
    if curve_editor_state.placement == CurveEditorPlacement::Window {
        build_placement_toggle(ui, curve_editor_state);
        ui.same_line();
        ui.text_disabled("The curve editor is open in its own window");
        return;
    }
    build_curve_editor_tab(
        ui,
        world,
        state,
        clip_library,
        curve_editor_state,
        curve_buffer,
        pose_library,
    );
}

pub(super) fn draw_timeline_tab(
    ui: &imgui::Ui,
    world: &World,
    state: &mut TimelineState,
    interaction: &mut TimelineInteractionState,
    clip_library: &ClipLibrary,
    curve_editor_state: &mut CurveEditorState,
    clip_track_snapshot: &ClipTrackSnapshot,
) {
    handle_middle_drag_pan(ui, world, state);
    build_timeline_content(
        ui,
        world,
        state,
        interaction,
        clip_library,
        curve_editor_state,
        clip_track_snapshot,
    );

    let clip_duration = timeline_effective_duration(state, clip_library);
    handle_timeline_shortcuts(ui, world, state);
    handle_mouse_wheel_zoom(ui, world, state, clip_duration);
}

pub(super) fn build_timeline_content(
    ui: &imgui::Ui,
    world: &World,
    state: &mut TimelineState,
    interaction: &mut TimelineInteractionState,
    clip_library: &ClipLibrary,
    curve_editor_state: &mut CurveEditorState,
    clip_track_snapshot: &ClipTrackSnapshot,
) {
    let content_region = ui.content_region_avail();
    let visible_width = (content_region[0] - TRACK_LABEL_WIDTH).max(1.0);
    state.last_visible_width = visible_width;

    let duration = timeline_effective_duration(state, clip_library);
    let pixels_per_second = PIXELS_PER_SECOND * state.zoom_level;
    let timeline_width = (duration * pixels_per_second).max(visible_width);

    let ruler_child_height = TIME_RULER_HEIGHT + ui.text_line_height_with_spacing() + 4.0;
    let synced_scroll_x = state.scroll_offset;

    ui.child_window("timeline_ruler")
        .size([content_region[0], ruler_child_height])
        .scroll_bar(false)
        .horizontal_scrollbar(true)
        .build(ui, || {
            ui.set_scroll_x(synced_scroll_x);
            build_time_ruler_with_scrub(ui, world, state, interaction, timeline_width, duration);
        });
    ui.separator();

    let remaining = ui.content_region_avail();
    ui.child_window("timeline_tracks")
        .size(remaining)
        .horizontal_scrollbar(true)
        .build(ui, || {
            if state.pan_pending_delta_x.abs() >= 0.01 {
                let new_scroll = (ui.scroll_x() + state.pan_pending_delta_x).max(0.0);
                ui.set_scroll_x(new_scroll);
                state.pan_pending_delta_x = 0.0;
            }
            if !clip_track_snapshot.entries.is_empty() {
                build_clip_tracks_section(
                    ui,
                    world,
                    state,
                    interaction,
                    clip_library,
                    curve_editor_state,
                    clip_track_snapshot,
                    timeline_width,
                );
            }
        });
}

pub(super) fn build_timeline_window(
    ui: &imgui::Ui,
    world: &World,
    assets: &AssetStorage,
    _: &GraphicsResources,
) {
    let clip_track_snapshot = {
        let clip_library = world.resource::<ClipLibrary>();
        query_clip_tracks(world, &clip_library, assets)
    };

    let mut timeline_state = world.resource_mut::<TimelineState>();
    let mut interaction = world.resource_mut::<TimelineInteractionState>();
    let clip_library = world.resource::<ClipLibrary>();
    let mut curve_editor = world.resource_mut::<CurveEditorState>();
    let layout = world.resource::<LayoutSnapshot>();
    let curve_buffer = world.resource::<CurveEditorBuffer>();
    let mut pose_library = world.resource_mut::<PoseLibrary>();
    draw_timeline_window(
        ui,
        world,
        &mut timeline_state,
        &mut interaction,
        &clip_library,
        &mut curve_editor,
        &clip_track_snapshot,
        &layout,
        &curve_buffer,
        &mut pose_library,
    );
}

crate::ui_window!("timeline", Bottom, 0, build_timeline_window);
