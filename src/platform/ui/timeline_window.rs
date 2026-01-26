use imgui::Condition;

use crate::animation::editable::{EditableAnimationClip, EditableClipManager, PropertyCurve};
use crate::animation::BoneId;
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::TimelineState;

use super::CurveEditorState;

const TRACK_LABEL_WIDTH: f32 = 150.0;
const TRACK_HEIGHT: f32 = 24.0;
const CURVE_HEIGHT: f32 = 60.0;
const TIME_RULER_HEIGHT: f32 = 30.0;
const MAX_VISIBLE_TRACKS: usize = 10;
const PIXELS_PER_SECOND: f32 = 80.0;
const PLAYHEAD_HANDLE_SIZE: f32 = 10.0;

pub fn build_timeline_window(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip_manager: &EditableClipManager,
    curve_editor_state: &mut CurveEditorState,
) {
    let display_size = ui.io().display_size;
    let timeline_height = 300.0;
    let debug_window_height = 250.0;
    let timeline_y = display_size[1] - debug_window_height - timeline_height;

    ui.window("Timeline")
        .position([0.0, timeline_y], Condition::Always)
        .size([display_size[0], timeline_height], Condition::Always)
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .build(|| {
            build_transport_controls(ui, ui_events, state, clip_manager, curve_editor_state);
            ui.separator();
            build_timeline_content(ui, ui_events, state, clip_manager);
        });
}

fn build_transport_controls(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip_manager: &EditableClipManager,
    curve_editor_state: &mut CurveEditorState,
) {
    if state.playing {
        if ui.button("||") {
            ui_events.send(UIEvent::TimelinePause);
        }
    } else if ui.button(">") {
        ui_events.send(UIEvent::TimelinePlay);
    }

    ui.same_line();
    if ui.button("[]") {
        ui_events.send(UIEvent::TimelineStop);
    }

    ui.same_line();
    let mut looping = state.looping;
    if ui.checkbox("Loop", &mut looping) {
        ui_events.send(UIEvent::TimelineToggleLoop);
    }

    ui.same_line();
    let current_clip = state
        .current_clip_id
        .and_then(|id| clip_manager.get(id));

    let duration = current_clip.map(|c| c.duration).unwrap_or(0.0);

    ui.text(format!(
        "Time: {:.2}s / {:.2}s",
        state.current_time, duration
    ));

    ui.same_line();
    if ui.button("-") {
        ui_events.send(UIEvent::TimelineZoomOut);
    }
    ui.same_line();
    if ui.button("+") {
        ui_events.send(UIEvent::TimelineZoomIn);
    }
    ui.same_line();
    ui.text(format!("Zoom: {:.1}x", state.zoom_level));

    ui.same_line();
    if ui.button("Curve Editor") {
        curve_editor_state.is_open = true;
        if let Some(first_bone_id) = current_clip.and_then(|c| c.tracks.keys().next().copied()) {
            curve_editor_state.selected_bone_id = Some(first_bone_id);
        }
    }

    build_clip_selector(ui, ui_events, state, clip_manager);
}

fn build_clip_selector(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip_manager: &EditableClipManager,
) {
    let clip_names = clip_manager.clip_names();

    if clip_names.is_empty() {
        ui.text("No clips available");
        return;
    }

    let current_name = state
        .current_clip_id
        .and_then(|id| clip_manager.get(id))
        .map(|c| c.name.as_str())
        .unwrap_or("Select Clip");

    ui.same_line();
    ui.set_next_item_width(150.0);

    if let Some(_token) = ui.begin_combo("##clip_select", current_name) {
        for (id, name) in &clip_names {
            let is_selected = state.current_clip_id == Some(*id);
            if ui.selectable_config(name).selected(is_selected).build() {
                ui_events.send(UIEvent::TimelineSelectClip(*id));
            }
        }
    }
}

fn build_timeline_content(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip_manager: &EditableClipManager,
) {
    let current_clip = match state.current_clip_id.and_then(|id| clip_manager.get(id)) {
        Some(clip) => clip,
        None => {
            ui.text("Select a clip to edit");
            return;
        }
    };

    let content_region = ui.content_region_avail();
    let pixels_per_second = PIXELS_PER_SECOND * state.zoom_level;
    let timeline_width = (current_clip.duration * pixels_per_second).max(content_region[0] - TRACK_LABEL_WIDTH);

    ui.child_window("timeline_content")
        .size(content_region)
        .build(|| {
            build_time_ruler_with_scrub(ui, ui_events, state, current_clip, timeline_width);
            ui.separator();
            build_tracks_area(ui, ui_events, state, current_clip, timeline_width);
        });
}

fn build_time_ruler_with_scrub(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip: &EditableAnimationClip,
    timeline_width: f32,
) {
    let cursor_pos = ui.cursor_screen_pos();
    let ruler_start_x = cursor_pos[0] + TRACK_LABEL_WIDTH;
    let ruler_width = timeline_width;
    let pixels_per_second = PIXELS_PER_SECOND * state.zoom_level;

    ui.text("Time:");
    ui.same_line_with_pos(TRACK_LABEL_WIDTH);

    let draw_list = ui.get_window_draw_list();

    draw_list
        .add_rect(
            [ruler_start_x, cursor_pos[1]],
            [ruler_start_x + ruler_width, cursor_pos[1] + TIME_RULER_HEIGHT],
            [0.2, 0.2, 0.25, 1.0],
        )
        .filled(true)
        .build();

    let tick_interval = calculate_tick_interval(state.zoom_level);
    let mut time = 0.0;
    while time <= clip.duration {
        let x = ruler_start_x + time * pixels_per_second;
        let is_major = (time / tick_interval).round() as i32 % 5 == 0;

        let tick_height = if is_major { 12.0 } else { 6.0 };
        let tick_color = if is_major {
            [0.7, 0.7, 0.7, 1.0]
        } else {
            [0.4, 0.4, 0.4, 1.0]
        };

        draw_list
            .add_line(
                [x, cursor_pos[1] + TIME_RULER_HEIGHT - tick_height],
                [x, cursor_pos[1] + TIME_RULER_HEIGHT],
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

    let playhead_x = ruler_start_x + state.current_time * pixels_per_second;
    draw_playhead_handle(&draw_list, playhead_x, cursor_pos[1], TIME_RULER_HEIGHT);

    let ruler_rect_min = [ruler_start_x, cursor_pos[1]];
    let ruler_rect_max = [ruler_start_x + ruler_width, cursor_pos[1] + TIME_RULER_HEIGHT];

    handle_scrub_interaction(ui, ui_events, ruler_rect_min, ruler_rect_max, clip.duration, pixels_per_second, ruler_start_x);

    ui.dummy([ruler_width + TRACK_LABEL_WIDTH, TIME_RULER_HEIGHT]);
}

fn draw_playhead_handle(
    draw_list: &imgui::DrawListMut,
    x: f32,
    y: f32,
    ruler_height: f32,
) {
    draw_list
        .add_triangle(
            [x - PLAYHEAD_HANDLE_SIZE, y],
            [x + PLAYHEAD_HANDLE_SIZE, y],
            [x, y + PLAYHEAD_HANDLE_SIZE + 4.0],
            [1.0, 0.3, 0.3, 1.0],
        )
        .filled(true)
        .build();

    draw_list
        .add_line(
            [x, y + PLAYHEAD_HANDLE_SIZE],
            [x, y + ruler_height],
            [1.0, 0.3, 0.3, 1.0],
        )
        .thickness(2.0)
        .build();
}

fn handle_scrub_interaction(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    rect_min: [f32; 2],
    rect_max: [f32; 2],
    duration: f32,
    pixels_per_second: f32,
    ruler_start_x: f32,
) {
    let mouse_pos = ui.io().mouse_pos;
    let is_mouse_in_ruler = mouse_pos[0] >= rect_min[0]
        && mouse_pos[0] <= rect_max[0]
        && mouse_pos[1] >= rect_min[1]
        && mouse_pos[1] <= rect_max[1];

    let is_dragging = ui.io().mouse_down[0];

    if is_mouse_in_ruler && is_dragging {
        let relative_x = mouse_pos[0] - ruler_start_x;
        let new_time = (relative_x / pixels_per_second).clamp(0.0, duration);
        ui_events.send(UIEvent::TimelineSetTime(new_time));
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

fn build_tracks_area(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip: &EditableAnimationClip,
    timeline_width: f32,
) {
    let content_region = ui.content_region_avail();

    ui.child_window("tracks_scroll")
        .size([content_region[0], content_region[1]])
        .horizontal_scrollbar(true)
        .build(|| {
            build_tracks(ui, ui_events, state, clip, timeline_width);
        });
}

fn build_tracks(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip: &EditableAnimationClip,
    timeline_width: f32,
) {
    let mut sorted_bone_ids: Vec<BoneId> = clip.tracks.keys().copied().collect();
    sorted_bone_ids.sort();

    let total_tracks = sorted_bone_ids.len();
    let visible_tracks: Vec<_> = sorted_bone_ids.into_iter().take(MAX_VISIBLE_TRACKS).collect();

    if total_tracks > MAX_VISIBLE_TRACKS {
        ui.text_colored(
            [1.0, 0.7, 0.3, 1.0],
            &format!("Showing {}/{} tracks", MAX_VISIBLE_TRACKS, total_tracks),
        );
    }

    for bone_id in visible_tracks {
        if let Some(track) = clip.tracks.get(&bone_id) {
            let is_expanded = state.is_track_expanded(bone_id);

            build_track_row(
                ui,
                ui_events,
                state,
                bone_id,
                &track.bone_name,
                is_expanded,
                track.has_any_keyframes(),
                clip.duration,
                timeline_width,
                track,
            );
        }
    }
}

fn build_track_row(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    bone_id: BoneId,
    bone_name: &str,
    is_expanded: bool,
    has_keyframes: bool,
    duration: f32,
    timeline_width: f32,
    track: &crate::animation::editable::BoneTrack,
) {
    let expand_char = if is_expanded { "v" } else { ">" };

    if ui.small_button(&format!("{}##{}", expand_char, bone_id)) {
        ui_events.send(UIEvent::TimelineToggleTrack(bone_id));
    }

    ui.same_line();

    let display_name = if bone_name.len() > 15 {
        format!("{}...", &bone_name[..12])
    } else {
        bone_name.to_string()
    };

    let color = if has_keyframes {
        [1.0, 1.0, 1.0, 1.0]
    } else {
        [0.5, 0.5, 0.5, 1.0]
    };

    ui.text_colored(color, &display_name);

    ui.same_line_with_pos(TRACK_LABEL_WIDTH);

    let row_height = if is_expanded { CURVE_HEIGHT } else { TRACK_HEIGHT };

    ui.child_window(&format!("track_area_{}", bone_id))
        .size([timeline_width.max(200.0), row_height])
        .build(|| {
            if is_expanded {
                draw_curve_area(ui, state, track, duration, timeline_width);
            } else {
                draw_keyframe_markers(ui, state, track, duration, timeline_width);
            }
        });
}

fn draw_keyframe_markers(
    ui: &imgui::Ui,
    state: &TimelineState,
    track: &crate::animation::editable::BoneTrack,
    duration: f32,
    timeline_width: f32,
) {
    if duration <= 0.0 {
        return;
    }

    let draw_list = ui.get_window_draw_list();
    let cursor_pos = ui.cursor_screen_pos();
    let pixels_per_second = PIXELS_PER_SECOND * state.zoom_level;

    draw_list
        .add_rect(
            cursor_pos,
            [cursor_pos[0] + timeline_width, cursor_pos[1] + TRACK_HEIGHT],
            [0.2, 0.2, 0.2, 1.0],
        )
        .filled(true)
        .build();

    let keyframe_times = track.collect_all_keyframe_times();
    let marker_count = keyframe_times.len().min(100);

    for time in keyframe_times.into_iter().take(marker_count) {
        let x = cursor_pos[0] + time * pixels_per_second;
        let y_center = cursor_pos[1] + TRACK_HEIGHT * 0.5;

        draw_list
            .add_rect(
                [x - 2.0, y_center - 6.0],
                [x + 2.0, y_center + 6.0],
                [0.9, 0.7, 0.2, 1.0],
            )
            .filled(true)
            .build();
    }

    draw_track_playhead(&draw_list, cursor_pos, state.current_time, pixels_per_second, TRACK_HEIGHT);
}

fn draw_curve_area(
    ui: &imgui::Ui,
    state: &TimelineState,
    track: &crate::animation::editable::BoneTrack,
    duration: f32,
    timeline_width: f32,
) {
    if duration <= 0.0 {
        return;
    }

    let draw_list = ui.get_window_draw_list();
    let cursor_pos = ui.cursor_screen_pos();
    let pixels_per_second = PIXELS_PER_SECOND * state.zoom_level;

    draw_list
        .add_rect(
            cursor_pos,
            [cursor_pos[0] + timeline_width, cursor_pos[1] + CURVE_HEIGHT],
            [0.15, 0.15, 0.18, 1.0],
        )
        .filled(true)
        .build();

    let center_y = cursor_pos[1] + CURVE_HEIGHT * 0.5;
    draw_list
        .add_line(
            [cursor_pos[0], center_y],
            [cursor_pos[0] + timeline_width, center_y],
            [0.3, 0.3, 0.3, 1.0],
        )
        .build();

    let curves_to_draw: Vec<(&PropertyCurve, [f32; 4])> = vec![
        (&track.translation_x, [1.0, 0.3, 0.3, 1.0]),
        (&track.translation_y, [0.3, 1.0, 0.3, 1.0]),
        (&track.translation_z, [0.3, 0.3, 1.0, 1.0]),
        (&track.rotation_x, [1.0, 0.5, 0.5, 0.8]),
        (&track.rotation_y, [0.5, 1.0, 0.5, 0.8]),
        (&track.rotation_z, [0.5, 0.5, 1.0, 0.8]),
    ];

    for (curve, color) in curves_to_draw {
        if curve.is_empty() {
            continue;
        }

        draw_single_curve(
            &draw_list,
            cursor_pos,
            curve,
            color,
            duration,
            pixels_per_second,
            CURVE_HEIGHT,
            timeline_width,
        );
    }

    draw_track_playhead(&draw_list, cursor_pos, state.current_time, pixels_per_second, CURVE_HEIGHT);
}

fn draw_single_curve(
    draw_list: &imgui::DrawListMut,
    cursor_pos: [f32; 2],
    curve: &PropertyCurve,
    color: [f32; 4],
    duration: f32,
    pixels_per_second: f32,
    height: f32,
    timeline_width: f32,
) {
    if curve.keyframes.is_empty() {
        return;
    }

    let sample_count = calculate_sample_count(timeline_width);

    let (min_val, max_val) = calculate_value_range(curve);
    let value_range = (max_val - min_val).max(0.001);

    let step = duration / sample_count as f32;
    let mut prev_point: Option<[f32; 2]> = None;

    for i in 0..=sample_count {
        let time = (i as f32) * step;
        if let Some(value) = curve.sample(time) {
            let x = cursor_pos[0] + time * pixels_per_second;
            let normalized = (value - min_val) / value_range;
            let y = cursor_pos[1] + height - (normalized * height * 0.8 + height * 0.1);

            let current_point = [x, y];

            if let Some(prev) = prev_point {
                draw_list.add_line(prev, current_point, color).build();
            }

            prev_point = Some(current_point);
        }
    }

    for kf in &curve.keyframes {
        let x = cursor_pos[0] + kf.time * pixels_per_second;
        let normalized = (kf.value - min_val) / value_range;
        let y = cursor_pos[1] + height - (normalized * height * 0.8 + height * 0.1);

        draw_list
            .add_circle([x, y], 4.0, color)
            .filled(true)
            .build();
    }
}

fn calculate_value_range(curve: &PropertyCurve) -> (f32, f32) {
    let mut min_val = f32::MAX;
    let mut max_val = f32::MIN;

    for kf in &curve.keyframes {
        min_val = min_val.min(kf.value);
        max_val = max_val.max(kf.value);
    }

    if min_val == max_val {
        min_val -= 0.5;
        max_val += 0.5;
    }

    (min_val, max_val)
}

fn calculate_sample_count(width: f32) -> usize {
    let base_samples = 30;
    let samples_per_100px = 10;
    let additional = ((width / 100.0) as usize) * samples_per_100px;
    (base_samples + additional).min(150)
}

fn draw_track_playhead(
    draw_list: &imgui::DrawListMut,
    cursor_pos: [f32; 2],
    current_time: f32,
    pixels_per_second: f32,
    height: f32,
) {
    let x = cursor_pos[0] + current_time * pixels_per_second;

    draw_list
        .add_line(
            [x, cursor_pos[1]],
            [x, cursor_pos[1] + height],
            [1.0, 0.2, 0.2, 0.8],
        )
        .thickness(2.0)
        .build();
}
