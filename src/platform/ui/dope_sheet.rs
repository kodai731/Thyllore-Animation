use crate::animation::editable::{EditableAnimationClip, PropertyType};
use crate::animation::BoneId;
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{
    SelectedKeyframe, SelectionModifier, TimelineState,
};

const DOPE_ROW_HEIGHT: f32 = 22.0;
const DOPE_SUB_ROW_HEIGHT: f32 = 18.0;
const DOPE_LABEL_WIDTH: f32 = 150.0;
const DOPE_PIXELS_PER_SECOND: f32 = 80.0;
const KEYFRAME_DIAMOND_SIZE: f32 = 5.0;
const RULER_HEIGHT: f32 = 24.0;
const MAX_VISIBLE_TRACKS: usize = 10;

const PROPERTY_COLORS: [[f32; 4]; 9] = [
    [0.9, 0.3, 0.3, 1.0],
    [0.3, 0.9, 0.3, 1.0],
    [0.3, 0.3, 0.9, 1.0],
    [0.9, 0.6, 0.3, 1.0],
    [0.3, 0.9, 0.6, 1.0],
    [0.6, 0.3, 0.9, 1.0],
    [0.9, 0.9, 0.3, 1.0],
    [0.3, 0.9, 0.9, 1.0],
    [0.9, 0.3, 0.9, 1.0],
];

pub fn build_dope_sheet(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    timeline_state: &TimelineState,
    clip: &EditableAnimationClip,
    content_width: f32,
    content_height: f32,
) {
    let pixels_per_second = DOPE_PIXELS_PER_SECOND * timeline_state.zoom_level;
    let sheet_width =
        (clip.duration * pixels_per_second).max(content_width - DOPE_LABEL_WIDTH);

    ui.child_window("dope_sheet_area")
        .size([content_width, content_height])
        .horizontal_scrollbar(true)
        .build(|| {
            draw_dope_sheet_ruler(
                ui,
                timeline_state,
                clip,
                sheet_width,
                pixels_per_second,
            );

            draw_summary_row(
                ui,
                timeline_state,
                clip,
                sheet_width,
                pixels_per_second,
            );

            draw_bone_rows(
                ui,
                ui_events,
                timeline_state,
                clip,
                sheet_width,
                pixels_per_second,
            );

            if ui.is_mouse_clicked(imgui::MouseButton::Right) {
                ui.open_popup("dope_sheet_context");
            }
            build_dope_sheet_context_menu(ui, ui_events, timeline_state);
        });
}

fn draw_dope_sheet_ruler(
    ui: &imgui::Ui,
    state: &TimelineState,
    clip: &EditableAnimationClip,
    sheet_width: f32,
    pixels_per_second: f32,
) {
    let cursor_pos = ui.cursor_screen_pos();
    let ruler_x = cursor_pos[0] + DOPE_LABEL_WIDTH;
    let draw_list = ui.get_window_draw_list();

    draw_list
        .add_rect(
            [ruler_x, cursor_pos[1]],
            [ruler_x + sheet_width, cursor_pos[1] + RULER_HEIGHT],
            [0.2, 0.2, 0.25, 1.0],
        )
        .filled(true)
        .build();

    let tick_interval = calculate_tick_interval(state.zoom_level);
    let mut time = 0.0;
    while time <= clip.duration {
        let x = ruler_x + time * pixels_per_second;
        let is_major = (time / tick_interval).round() as i32 % 5 == 0;

        let tick_h = if is_major { 10.0 } else { 5.0 };
        let tick_color = if is_major {
            [0.7, 0.7, 0.7, 1.0]
        } else {
            [0.4, 0.4, 0.4, 1.0]
        };

        draw_list
            .add_line(
                [x, cursor_pos[1] + RULER_HEIGHT - tick_h],
                [x, cursor_pos[1] + RULER_HEIGHT],
                tick_color,
            )
            .build();

        if is_major {
            draw_list.add_text(
                [x - 10.0, cursor_pos[1] + 2.0],
                [0.8, 0.8, 0.8, 1.0],
                &format!("{:.1}s", time),
            );
        }

        time += tick_interval;
    }

    let playhead_x = ruler_x + state.current_time * pixels_per_second;
    draw_list
        .add_line(
            [playhead_x, cursor_pos[1]],
            [playhead_x, cursor_pos[1] + RULER_HEIGHT],
            [1.0, 0.3, 0.3, 1.0],
        )
        .thickness(2.0)
        .build();

    ui.text("Time:");
    ui.same_line_with_pos(DOPE_LABEL_WIDTH);
    ui.dummy([sheet_width, RULER_HEIGHT]);
}

fn draw_summary_row(
    ui: &imgui::Ui,
    state: &TimelineState,
    clip: &EditableAnimationClip,
    sheet_width: f32,
    pixels_per_second: f32,
) {
    let cursor_pos = ui.cursor_screen_pos();
    let row_x = cursor_pos[0] + DOPE_LABEL_WIDTH;
    let draw_list = ui.get_window_draw_list();

    draw_list
        .add_rect(
            [row_x, cursor_pos[1]],
            [row_x + sheet_width, cursor_pos[1] + DOPE_ROW_HEIGHT],
            [0.18, 0.18, 0.22, 1.0],
        )
        .filled(true)
        .build();

    let mut all_times: Vec<f32> = Vec::new();
    for track in clip.tracks.values() {
        let times = track.collect_all_keyframe_times();
        all_times.extend(times);
    }

    all_times.sort_by(|a, b| {
        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
    });
    all_times.dedup_by(|a, b| (*a - *b).abs() < 0.001);

    let y_center = cursor_pos[1] + DOPE_ROW_HEIGHT * 0.5;
    for time in &all_times {
        let x = row_x + time * pixels_per_second;
        draw_diamond(
            &draw_list,
            x,
            y_center,
            KEYFRAME_DIAMOND_SIZE,
            [0.8, 0.8, 0.8, 1.0],
        );
    }

    draw_playhead_line(
        &draw_list,
        row_x,
        cursor_pos[1],
        DOPE_ROW_HEIGHT,
        state.current_time,
        pixels_per_second,
    );

    ui.text_colored([0.7, 0.7, 0.7, 1.0], "Summary");
    ui.same_line_with_pos(DOPE_LABEL_WIDTH);
    ui.dummy([sheet_width, DOPE_ROW_HEIGHT]);
}

fn draw_bone_rows(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip: &EditableAnimationClip,
    sheet_width: f32,
    pixels_per_second: f32,
) {
    let mut sorted_bone_ids: Vec<BoneId> =
        clip.tracks.keys().copied().collect();
    sorted_bone_ids.sort();

    for bone_id in sorted_bone_ids.into_iter().take(MAX_VISIBLE_TRACKS) {
        let track = match clip.tracks.get(&bone_id) {
            Some(t) => t,
            None => continue,
        };

        let is_expanded = state.is_track_expanded(bone_id);

        if is_expanded {
            draw_bone_row_expanded(
                ui,
                ui_events,
                state,
                bone_id,
                &track.bone_name,
                track,
                sheet_width,
                pixels_per_second,
            );
        } else {
            draw_bone_row_collapsed(
                ui,
                ui_events,
                state,
                bone_id,
                &track.bone_name,
                track,
                sheet_width,
                pixels_per_second,
            );
        }
    }
}

fn draw_bone_row_collapsed(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    bone_id: BoneId,
    bone_name: &str,
    track: &crate::animation::editable::BoneTrack,
    sheet_width: f32,
    pixels_per_second: f32,
) {
    let cursor_pos = ui.cursor_screen_pos();
    let row_x = cursor_pos[0] + DOPE_LABEL_WIDTH;
    let draw_list = ui.get_window_draw_list();

    draw_list
        .add_rect(
            [row_x, cursor_pos[1]],
            [row_x + sheet_width, cursor_pos[1] + DOPE_ROW_HEIGHT],
            [0.15, 0.15, 0.18, 1.0],
        )
        .filled(true)
        .build();

    let y_center = cursor_pos[1] + DOPE_ROW_HEIGHT * 0.5;
    let keyframe_times = track.collect_all_keyframe_times();

    for time in keyframe_times.iter().take(100) {
        let x = row_x + time * pixels_per_second;
        draw_diamond(
            &draw_list,
            x,
            y_center,
            KEYFRAME_DIAMOND_SIZE,
            [0.9, 0.7, 0.2, 1.0],
        );
    }

    draw_playhead_line(
        &draw_list,
        row_x,
        cursor_pos[1],
        DOPE_ROW_HEIGHT,
        state.current_time,
        pixels_per_second,
    );

    handle_collapsed_row_click(
        ui,
        ui_events,
        state,
        bone_id,
        track,
        row_x,
        cursor_pos[1],
        sheet_width,
        pixels_per_second,
    );

    if ui.small_button(&format!(">##{}", bone_id)) {
        ui_events.send(UIEvent::TimelineToggleTrack(bone_id));
    }

    ui.same_line();
    let label = truncate_name(bone_name, 15);
    ui.text_colored([1.0, 1.0, 1.0, 1.0], &label);
    ui.same_line_with_pos(DOPE_LABEL_WIDTH);
    ui.dummy([sheet_width, DOPE_ROW_HEIGHT]);
}

fn draw_bone_row_expanded(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    bone_id: BoneId,
    bone_name: &str,
    track: &crate::animation::editable::BoneTrack,
    sheet_width: f32,
    pixels_per_second: f32,
) {
    if ui.small_button(&format!("v##{}", bone_id)) {
        ui_events.send(UIEvent::TimelineToggleTrack(bone_id));
    }
    ui.same_line();
    let label = truncate_name(bone_name, 15);
    ui.text_colored([0.9, 0.9, 0.5, 1.0], &label);

    let all_properties = [
        PropertyType::TranslationX,
        PropertyType::TranslationY,
        PropertyType::TranslationZ,
        PropertyType::RotationX,
        PropertyType::RotationY,
        PropertyType::RotationZ,
        PropertyType::ScaleX,
        PropertyType::ScaleY,
        PropertyType::ScaleZ,
    ];

    for (prop_idx, prop) in all_properties.iter().enumerate() {
        let curve = track.get_curve(*prop);
        if curve.is_empty() {
            continue;
        }

        let cursor_pos = ui.cursor_screen_pos();
        let row_x = cursor_pos[0] + DOPE_LABEL_WIDTH;
        let draw_list = ui.get_window_draw_list();

        let bg_color = if prop_idx % 2 == 0 {
            [0.13, 0.13, 0.16, 1.0]
        } else {
            [0.16, 0.16, 0.19, 1.0]
        };

        draw_list
            .add_rect(
                [row_x, cursor_pos[1]],
                [
                    row_x + sheet_width,
                    cursor_pos[1] + DOPE_SUB_ROW_HEIGHT,
                ],
                bg_color,
            )
            .filled(true)
            .build();

        let y_center = cursor_pos[1] + DOPE_SUB_ROW_HEIGHT * 0.5;
        let prop_color = PROPERTY_COLORS[prop_idx % PROPERTY_COLORS.len()];

        for kf in &curve.keyframes {
            let x = row_x + kf.time * pixels_per_second;

            let sel_key = SelectedKeyframe::new(bone_id, *prop, kf.id);
            let is_selected = state.is_keyframe_selected(&sel_key);

            let color = if is_selected {
                [1.0, 1.0, 0.2, 1.0]
            } else {
                prop_color
            };

            draw_diamond(
                &draw_list,
                x,
                y_center,
                KEYFRAME_DIAMOND_SIZE - 1.0,
                color,
            );
        }

        draw_playhead_line(
            &draw_list,
            row_x,
            cursor_pos[1],
            DOPE_SUB_ROW_HEIGHT,
            state.current_time,
            pixels_per_second,
        );

        handle_expanded_row_click(
            ui,
            ui_events,
            state,
            bone_id,
            *prop,
            curve,
            row_x,
            cursor_pos[1],
            sheet_width,
            pixels_per_second,
        );

        ui.text_colored(prop_color, &format!("  {}", prop.short_name()));
        ui.same_line_with_pos(DOPE_LABEL_WIDTH);
        ui.dummy([sheet_width, DOPE_SUB_ROW_HEIGHT]);
    }
}

fn handle_collapsed_row_click(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    bone_id: BoneId,
    track: &crate::animation::editable::BoneTrack,
    row_x: f32,
    row_y: f32,
    sheet_width: f32,
    pixels_per_second: f32,
) {
    if !ui.is_mouse_clicked(imgui::MouseButton::Left) {
        return;
    }

    let mouse = ui.io().mouse_pos;
    if mouse[0] < row_x
        || mouse[0] > row_x + sheet_width
        || mouse[1] < row_y
        || mouse[1] > row_y + DOPE_ROW_HEIGHT
    {
        return;
    }

    let click_time = (mouse[0] - row_x) / pixels_per_second;
    let modifier = determine_selection_modifier(ui);

    let all_properties = [
        PropertyType::TranslationX,
        PropertyType::TranslationY,
        PropertyType::TranslationZ,
        PropertyType::RotationX,
        PropertyType::RotationY,
        PropertyType::RotationZ,
        PropertyType::ScaleX,
        PropertyType::ScaleY,
        PropertyType::ScaleZ,
    ];

    let threshold = 5.0 / pixels_per_second;
    for prop in &all_properties {
        let curve = track.get_curve(*prop);
        for kf in &curve.keyframes {
            if (kf.time - click_time).abs() < threshold {
                ui_events.send(UIEvent::TimelineSelectKeyframe {
                    bone_id,
                    property_type: *prop,
                    keyframe_id: kf.id,
                    modifier,
                });
                return;
            }
        }
    }
}

fn handle_expanded_row_click(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    _state: &TimelineState,
    bone_id: BoneId,
    property_type: PropertyType,
    curve: &crate::animation::editable::PropertyCurve,
    row_x: f32,
    row_y: f32,
    sheet_width: f32,
    pixels_per_second: f32,
) {
    if !ui.is_mouse_clicked(imgui::MouseButton::Left) {
        return;
    }

    let mouse = ui.io().mouse_pos;
    if mouse[0] < row_x
        || mouse[0] > row_x + sheet_width
        || mouse[1] < row_y
        || mouse[1] > row_y + DOPE_SUB_ROW_HEIGHT
    {
        return;
    }

    let click_time = (mouse[0] - row_x) / pixels_per_second;
    let threshold = 5.0 / pixels_per_second;
    let modifier = determine_selection_modifier(ui);

    for kf in &curve.keyframes {
        if (kf.time - click_time).abs() < threshold {
            ui_events.send(UIEvent::TimelineSelectKeyframe {
                bone_id,
                property_type,
                keyframe_id: kf.id,
                modifier,
            });
            return;
        }
    }
}

pub fn build_dope_sheet_context_menu(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    timeline_state: &TimelineState,
) {
    if let Some(_token) = ui.begin_popup("dope_sheet_context") {
        let has_selection = !timeline_state.selected_keyframes.is_empty();

        if has_selection {
            if ui.selectable("Copy") {
                ui_events.send(UIEvent::TimelineCopyKeyframes);
            }
        } else {
            ui.text_disabled("Copy");
        }

        if ui.selectable("Paste") {
            ui_events.send(UIEvent::TimelinePasteKeyframes {
                paste_time: timeline_state.current_time,
            });
        }

        if ui.selectable("Mirror Paste") {
            ui_events.send(UIEvent::TimelineMirrorPaste {
                paste_time: timeline_state.current_time,
            });
        }

        ui.separator();

        if has_selection {
            if ui.selectable("Delete") {
                ui_events.send(UIEvent::TimelineDeleteSelectedKeyframes);
            }
        } else {
            ui.text_disabled("Delete");
        }
    }
}

fn determine_selection_modifier(ui: &imgui::Ui) -> SelectionModifier {
    let io = ui.io();
    if io.key_ctrl {
        SelectionModifier::Toggle
    } else if io.key_shift {
        SelectionModifier::Add
    } else {
        SelectionModifier::Replace
    }
}

fn draw_diamond(
    draw_list: &imgui::DrawListMut,
    x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
) {
    let points = vec![
        [x, y - size],
        [x + size, y],
        [x, y + size],
        [x - size, y],
    ];

    draw_list
        .add_polyline(points, color)
        .filled(true)
        .build();
}

fn draw_playhead_line(
    draw_list: &imgui::DrawListMut,
    row_x: f32,
    row_y: f32,
    row_height: f32,
    current_time: f32,
    pixels_per_second: f32,
) {
    let px = row_x + current_time * pixels_per_second;
    draw_list
        .add_line(
            [px, row_y],
            [px, row_y + row_height],
            [1.0, 0.3, 0.3, 0.6],
        )
        .build();
}

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() > max_len {
        format!("{}...", &name[..max_len.saturating_sub(3)])
    } else {
        name.to_string()
    }
}

fn calculate_tick_interval(zoom_level: f32) -> f32 {
    if zoom_level < 0.5 {
        1.0
    } else if zoom_level < 1.0 {
        0.5
    } else if zoom_level < 2.0 {
        0.25
    } else {
        0.1
    }
}
