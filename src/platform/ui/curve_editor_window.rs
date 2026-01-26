use imgui::Condition;

use crate::animation::editable::{EditableClipManager, PropertyCurve, PropertyType, KeyframeId};
use crate::animation::BoneId;
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::TimelineState;

const MIN_WINDOW_WIDTH: f32 = 400.0;
const MIN_WINDOW_HEIGHT: f32 = 300.0;
const TRACK_LIST_WIDTH: f32 = 180.0;
const TIME_RULER_HEIGHT: f32 = 30.0;
const CURVE_PADDING: f32 = 10.0;
const KEYFRAME_HIT_RADIUS: f32 = 8.0;

#[derive(Clone, Debug)]
pub struct SelectedKeyframe {
    pub property_type: PropertyType,
    pub keyframe_id: KeyframeId,
    pub original_time: f32,
    pub original_value: f32,
}

pub struct CurveEditorState {
    pub is_open: bool,
    pub selected_bone_id: Option<BoneId>,
    pub show_translation: bool,
    pub show_rotation: bool,
    pub show_scale: bool,
    pub window_size: [f32; 2],
    pub selected_keyframe: Option<SelectedKeyframe>,
    pub is_dragging_keyframe: bool,
    pub drag_start_mouse_pos: [f32; 2],
}

impl Default for CurveEditorState {
    fn default() -> Self {
        Self {
            is_open: false,
            selected_bone_id: None,
            show_translation: true,
            show_rotation: true,
            show_scale: false,
            window_size: [800.0, 500.0],
            selected_keyframe: None,
            is_dragging_keyframe: false,
            drag_start_mouse_pos: [0.0, 0.0],
        }
    }
}

pub fn build_curve_editor_window(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    timeline_state: &TimelineState,
    clip_manager: &EditableClipManager,
    editor_state: &mut CurveEditorState,
) {
    if !editor_state.is_open {
        return;
    }

    let display_size = ui.io().display_size;
    let initial_pos = [
        (display_size[0] - editor_state.window_size[0]) * 0.5,
        (display_size[1] - editor_state.window_size[1]) * 0.5,
    ];

    let mut is_open = editor_state.is_open;

    ui.window("Curve Editor")
        .position(initial_pos, Condition::FirstUseEver)
        .size(editor_state.window_size, Condition::FirstUseEver)
        .size_constraints([MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT], [display_size[0], display_size[1]])
        .opened(&mut is_open)
        .build(|| {
            editor_state.window_size = ui.window_size();

            build_toolbar(ui, editor_state);
            ui.separator();

            let content_region = ui.content_region_avail();

            ui.child_window("track_list")
                .size([TRACK_LIST_WIDTH, content_region[1]])
                .border(true)
                .build(|| {
                    build_track_list(ui, timeline_state, clip_manager, editor_state);
                });

            ui.same_line();

            ui.child_window("curve_view")
                .size([content_region[0] - TRACK_LIST_WIDTH - 10.0, content_region[1]])
                .border(true)
                .horizontal_scrollbar(true)
                .build(|| {
                    build_curve_view(ui, ui_events, timeline_state, clip_manager, editor_state);
                });
        });

    editor_state.is_open = is_open;
}

fn build_toolbar(ui: &imgui::Ui, state: &mut CurveEditorState) {
    ui.checkbox("Translation", &mut state.show_translation);
    ui.same_line();
    ui.checkbox("Rotation", &mut state.show_rotation);
    ui.same_line();
    ui.checkbox("Scale", &mut state.show_scale);
}

fn build_track_list(
    ui: &imgui::Ui,
    timeline_state: &TimelineState,
    clip_manager: &EditableClipManager,
    editor_state: &mut CurveEditorState,
) {
    let clip = match timeline_state.current_clip_id.and_then(|id| clip_manager.get(id)) {
        Some(c) => c,
        None => {
            ui.text("No clip selected");
            return;
        }
    };

    ui.text("Bones:");
    ui.separator();

    let mut sorted_bone_ids: Vec<BoneId> = clip.tracks.keys().copied().collect();
    sorted_bone_ids.sort();

    for bone_id in sorted_bone_ids {
        if let Some(track) = clip.tracks.get(&bone_id) {
            let is_selected = editor_state.selected_bone_id == Some(bone_id);
            let label = if track.bone_name.len() > 18 {
                format!("{}...", &track.bone_name[..15])
            } else {
                track.bone_name.clone()
            };

            if ui.selectable_config(&label).selected(is_selected).build() {
                editor_state.selected_bone_id = Some(bone_id);
            }
        }
    }
}

fn build_curve_view(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    timeline_state: &TimelineState,
    clip_manager: &EditableClipManager,
    editor_state: &mut CurveEditorState,
) {
    let clip = match timeline_state.current_clip_id.and_then(|id| clip_manager.get(id)) {
        Some(c) => c,
        None => {
            ui.text("No clip selected");
            return;
        }
    };

    let bone_id = match editor_state.selected_bone_id {
        Some(id) => id,
        None => {
            ui.text("Select a bone from the list");
            return;
        }
    };

    let track = match clip.tracks.get(&bone_id) {
        Some(t) => t,
        None => {
            ui.text("Track not found");
            return;
        }
    };

    let content_region = ui.content_region_avail();
    let curve_area_width = content_region[0] - CURVE_PADDING * 2.0;
    let curve_area_height = content_region[1] - TIME_RULER_HEIGHT - CURVE_PADDING * 2.0;

    if curve_area_width <= 0.0 || curve_area_height <= 0.0 {
        return;
    }

    let sample_count = calculate_sample_count(curve_area_width);

    let draw_list = ui.get_window_draw_list();
    let cursor_pos = ui.cursor_screen_pos();

    let ruler_pos = [cursor_pos[0] + CURVE_PADDING, cursor_pos[1]];
    draw_time_ruler(&draw_list, ruler_pos, curve_area_width, clip.duration, timeline_state.zoom_level);

    let curve_origin = [cursor_pos[0] + CURVE_PADDING, cursor_pos[1] + TIME_RULER_HEIGHT + CURVE_PADDING];

    draw_list
        .add_rect(
            curve_origin,
            [curve_origin[0] + curve_area_width, curve_origin[1] + curve_area_height],
            [0.12, 0.12, 0.15, 1.0],
        )
        .filled(true)
        .build();

    draw_grid(&draw_list, curve_origin, curve_area_width, curve_area_height, clip.duration, timeline_state.zoom_level);

    let mut curves_to_draw: Vec<(&PropertyCurve, [f32; 4], &str)> = Vec::new();

    if editor_state.show_translation {
        curves_to_draw.push((&track.translation_x, [1.0, 0.3, 0.3, 1.0], "Pos.X"));
        curves_to_draw.push((&track.translation_y, [0.3, 1.0, 0.3, 1.0], "Pos.Y"));
        curves_to_draw.push((&track.translation_z, [0.3, 0.3, 1.0, 1.0], "Pos.Z"));
    }

    if editor_state.show_rotation {
        curves_to_draw.push((&track.rotation_x, [1.0, 0.6, 0.6, 1.0], "Rot.X"));
        curves_to_draw.push((&track.rotation_y, [0.6, 1.0, 0.6, 1.0], "Rot.Y"));
        curves_to_draw.push((&track.rotation_z, [0.6, 0.6, 1.0, 1.0], "Rot.Z"));
        curves_to_draw.push((&track.rotation_w, [0.8, 0.8, 0.8, 1.0], "Rot.W"));
    }

    if editor_state.show_scale {
        curves_to_draw.push((&track.scale_x, [1.0, 0.8, 0.4, 1.0], "Scl.X"));
        curves_to_draw.push((&track.scale_y, [0.8, 1.0, 0.4, 1.0], "Scl.Y"));
        curves_to_draw.push((&track.scale_z, [0.4, 0.8, 1.0, 1.0], "Scl.Z"));
    }

    let (global_min, global_max) = calculate_global_value_range(&curves_to_draw);

    for (curve, color, _name) in &curves_to_draw {
        if !curve.is_empty() {
            draw_curve_with_keyframes(
                &draw_list,
                curve_origin,
                curve,
                *color,
                clip.duration,
                curve_area_width,
                curve_area_height,
                sample_count,
                global_min,
                global_max,
            );
        }
    }

    if let Some(ref selected) = editor_state.selected_keyframe {
        for (curve, _, _) in &curves_to_draw {
            if curve.property_type == selected.property_type {
                if let Some(kf) = curve.get_keyframe(selected.keyframe_id) {
                    let value_range = (global_max - global_min).max(0.001);
                    let x = curve_origin[0] + (kf.time / clip.duration.max(0.001)) * curve_area_width;
                    let normalized = (kf.value - global_min) / value_range;
                    let y = curve_origin[1] + curve_area_height - normalized * curve_area_height;

                    draw_list
                        .add_circle([x, y], 8.0, [1.0, 1.0, 0.0, 1.0])
                        .thickness(2.0)
                        .build();
                }
                break;
            }
        }
    }

    let playhead_x = curve_origin[0] + (timeline_state.current_time / clip.duration.max(0.001)) * curve_area_width;
    draw_list
        .add_line(
            [playhead_x, curve_origin[1]],
            [playhead_x, curve_origin[1] + curve_area_height],
            [1.0, 0.2, 0.2, 1.0],
        )
        .thickness(2.0)
        .build();

    ui.set_cursor_screen_pos([cursor_pos[0], cursor_pos[1]]);
    let button_size = [curve_area_width + CURVE_PADDING * 2.0, curve_area_height + TIME_RULER_HEIGHT + CURVE_PADDING * 2.0];
    ui.invisible_button("curve_interaction_area", button_size);

    let is_hovered = ui.is_item_hovered();
    let mouse_pos = ui.io().mouse_pos;
    let mouse_clicked = ui.is_mouse_clicked(imgui::MouseButton::Left);
    let mouse_down = ui.io().mouse_down[0];
    let mouse_released = ui.is_mouse_released(imgui::MouseButton::Left);

    let in_ruler_area = mouse_pos[0] >= ruler_pos[0]
        && mouse_pos[0] <= ruler_pos[0] + curve_area_width
        && mouse_pos[1] >= ruler_pos[1]
        && mouse_pos[1] <= ruler_pos[1] + TIME_RULER_HEIGHT;

    let in_curve_area = mouse_pos[0] >= curve_origin[0]
        && mouse_pos[0] <= curve_origin[0] + curve_area_width
        && mouse_pos[1] >= curve_origin[1]
        && mouse_pos[1] <= curve_origin[1] + curve_area_height;

    if mouse_released {
        if editor_state.is_dragging_keyframe {
            if let Some(ref selected) = editor_state.selected_keyframe {
                let relative_x = mouse_pos[0] - curve_origin[0];
                let relative_y = mouse_pos[1] - curve_origin[1];
                let new_time = (relative_x / curve_area_width * clip.duration).clamp(0.0, clip.duration);
                let normalized_y = 1.0 - (relative_y / curve_area_height);
                let new_value = global_min + normalized_y * (global_max - global_min);

                if let Some(bone_id) = editor_state.selected_bone_id {
                    ui_events.send(UIEvent::TimelineMoveKeyframe {
                        bone_id,
                        property_type: selected.property_type.clone(),
                        keyframe_id: selected.keyframe_id,
                        new_time,
                        new_value,
                    });
                }
            }
        }
        editor_state.is_dragging_keyframe = false;
    }

    if is_hovered && mouse_clicked && in_ruler_area {
        let relative_x = mouse_pos[0] - ruler_pos[0];
        let new_time = (relative_x / curve_area_width * clip.duration).clamp(0.0, clip.duration);
        ui_events.send(UIEvent::TimelineSetTime(new_time));
    }

    if is_hovered && mouse_clicked && in_curve_area && !editor_state.is_dragging_keyframe {
        let hit_keyframe = find_keyframe_at_position(
            mouse_pos,
            curve_origin,
            &curves_to_draw,
            clip.duration,
            curve_area_width,
            curve_area_height,
            global_min,
            global_max,
        );

        if let Some((property_type, keyframe_id, time, value)) = hit_keyframe {
            editor_state.selected_keyframe = Some(SelectedKeyframe {
                property_type,
                keyframe_id,
                original_time: time,
                original_value: value,
            });
            editor_state.is_dragging_keyframe = true;
            editor_state.drag_start_mouse_pos = mouse_pos;
        } else {
            editor_state.selected_keyframe = None;
        }
    }

    if is_hovered && mouse_down && in_ruler_area && !editor_state.is_dragging_keyframe {
        let relative_x = mouse_pos[0] - ruler_pos[0];
        let new_time = (relative_x / curve_area_width * clip.duration).clamp(0.0, clip.duration);
        ui_events.send(UIEvent::TimelineSetTime(new_time));
    }

    if editor_state.is_dragging_keyframe {
        let preview_x = mouse_pos[0].clamp(curve_origin[0], curve_origin[0] + curve_area_width);
        let preview_y = mouse_pos[1].clamp(curve_origin[1], curve_origin[1] + curve_area_height);

        draw_list
            .add_circle([preview_x, preview_y], 7.0, [1.0, 1.0, 0.0, 1.0])
            .filled(true)
            .build();

        draw_list
            .add_circle([preview_x, preview_y], 7.0, [1.0, 1.0, 1.0, 1.0])
            .thickness(2.0)
            .build();

        let relative_x = preview_x - curve_origin[0];
        let relative_y = preview_y - curve_origin[1];
        let preview_time = (relative_x / curve_area_width * clip.duration).clamp(0.0, clip.duration);
        let normalized_y = 1.0 - (relative_y / curve_area_height);
        let preview_value = global_min + normalized_y * (global_max - global_min);

        draw_list.add_text(
            [preview_x + 10.0, preview_y - 10.0],
            [1.0, 1.0, 1.0, 1.0],
            &format!("t={:.2}s v={:.3}", preview_time, preview_value),
        );
    }

    draw_legend(ui, &curves_to_draw);
}

fn calculate_sample_count(width: f32) -> usize {
    let base_samples = 60;
    let samples_per_100px = 15;
    let additional = ((width / 100.0) as usize) * samples_per_100px;
    (base_samples + additional).min(200)
}

fn draw_time_ruler(
    draw_list: &imgui::DrawListMut,
    pos: [f32; 2],
    width: f32,
    duration: f32,
    zoom: f32,
) {
    draw_list
        .add_rect(
            pos,
            [pos[0] + width, pos[1] + TIME_RULER_HEIGHT],
            [0.18, 0.18, 0.22, 1.0],
        )
        .filled(true)
        .build();

    let tick_interval = if zoom < 0.5 { 1.0 } else if zoom < 1.5 { 0.5 } else { 0.25 };
    let pixels_per_second = width / duration.max(0.1);

    let mut time = 0.0;
    while time <= duration {
        let x = pos[0] + time * pixels_per_second;
        let is_major = (time / tick_interval).round() as i32 % 4 == 0;

        let tick_height = if is_major { 10.0 } else { 5.0 };
        draw_list
            .add_line(
                [x, pos[1] + TIME_RULER_HEIGHT - tick_height],
                [x, pos[1] + TIME_RULER_HEIGHT],
                [0.6, 0.6, 0.6, 1.0],
            )
            .build();

        if is_major {
            draw_list.add_text([x + 2.0, pos[1] + 2.0], [0.7, 0.7, 0.7, 1.0], &format!("{:.1}s", time));
        }

        time += tick_interval;
    }
}

fn draw_grid(
    draw_list: &imgui::DrawListMut,
    origin: [f32; 2],
    width: f32,
    height: f32,
    duration: f32,
    zoom: f32,
) {
    let grid_color = [0.25, 0.25, 0.28, 1.0];
    let pixels_per_second = width / duration.max(0.1);
    let time_step = if zoom < 0.5 { 1.0 } else if zoom < 1.5 { 0.5 } else { 0.25 };

    let mut time = 0.0;
    while time <= duration {
        let x = origin[0] + time * pixels_per_second;
        draw_list
            .add_line([x, origin[1]], [x, origin[1] + height], grid_color)
            .build();
        time += time_step;
    }

    let center_y = origin[1] + height * 0.5;
    draw_list
        .add_line([origin[0], center_y], [origin[0] + width, center_y], [0.35, 0.35, 0.38, 1.0])
        .build();
}

fn calculate_global_value_range(curves: &[(&PropertyCurve, [f32; 4], &str)]) -> (f32, f32) {
    let mut min_val = f32::MAX;
    let mut max_val = f32::MIN;

    for (curve, _, _) in curves {
        for kf in &curve.keyframes {
            min_val = min_val.min(kf.value);
            max_val = max_val.max(kf.value);
        }
    }

    if min_val == f32::MAX {
        min_val = -1.0;
        max_val = 1.0;
    } else if (max_val - min_val).abs() < 0.001 {
        min_val -= 0.5;
        max_val += 0.5;
    }

    let padding = (max_val - min_val) * 0.1;
    (min_val - padding, max_val + padding)
}

fn draw_curve_with_keyframes(
    draw_list: &imgui::DrawListMut,
    origin: [f32; 2],
    curve: &PropertyCurve,
    color: [f32; 4],
    duration: f32,
    width: f32,
    height: f32,
    sample_count: usize,
    min_val: f32,
    max_val: f32,
) {
    if curve.keyframes.is_empty() {
        return;
    }

    let value_range = (max_val - min_val).max(0.001);
    let step = duration / sample_count as f32;
    let mut prev_point: Option<[f32; 2]> = None;

    for i in 0..=sample_count {
        let time = (i as f32) * step;
        if let Some(value) = curve.sample(time) {
            let x = origin[0] + (time / duration.max(0.001)) * width;
            let normalized = (value - min_val) / value_range;
            let y = origin[1] + height - normalized * height;

            let point = [x, y];

            if let Some(prev) = prev_point {
                draw_list.add_line(prev, point, color).thickness(1.5).build();
            }

            prev_point = Some(point);
        }
    }

    for kf in &curve.keyframes {
        let x = origin[0] + (kf.time / duration.max(0.001)) * width;
        let normalized = (kf.value - min_val) / value_range;
        let y = origin[1] + height - normalized * height;

        draw_list
            .add_circle([x, y], 5.0, color)
            .filled(true)
            .build();

        draw_list
            .add_circle([x, y], 5.0, [1.0, 1.0, 1.0, 0.8])
            .build();
    }
}

fn draw_legend(ui: &imgui::Ui, curves: &[(&PropertyCurve, [f32; 4], &str)]) {
    ui.text("Legend: ");
    for (curve, color, name) in curves {
        if !curve.is_empty() {
            ui.same_line();
            ui.text_colored(*color, *name);
        }
    }
}

fn find_keyframe_at_position(
    mouse_pos: [f32; 2],
    curve_origin: [f32; 2],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    duration: f32,
    width: f32,
    height: f32,
    min_val: f32,
    max_val: f32,
) -> Option<(PropertyType, KeyframeId, f32, f32)> {
    let value_range = (max_val - min_val).max(0.001);

    for (curve, _, _) in curves {
        for kf in &curve.keyframes {
            let x = curve_origin[0] + (kf.time / duration.max(0.001)) * width;
            let normalized = (kf.value - min_val) / value_range;
            let y = curve_origin[1] + height - normalized * height;

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
