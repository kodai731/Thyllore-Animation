use std::collections::HashSet;

use imgui::Condition;

use crate::animation::editable::{KeyframeId, PropertyCurve, PropertyType};
use crate::ecs::resource::ClipLibrary;
use crate::animation::BoneId;
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::TimelineState;

const MIN_WINDOW_WIDTH: f32 = 400.0;
const MIN_WINDOW_HEIGHT: f32 = 300.0;
const TRACK_LIST_WIDTH: f32 = 180.0;
const TIME_RULER_HEIGHT: f32 = 30.0;
const CURVE_PADDING: f32 = 10.0;
const KEYFRAME_HIT_RADIUS: f32 = 8.0;
const Y_AXIS_WIDTH: f32 = 50.0;
const PAN_SPEED: f32 = 30.0;

const ALL_PROPERTY_TYPES: &[(PropertyType, [f32; 4], &str)] = &[
    (PropertyType::TranslationX, [1.0, 0.3, 0.3, 1.0], "Pos.X"),
    (PropertyType::TranslationY, [0.3, 1.0, 0.3, 1.0], "Pos.Y"),
    (PropertyType::TranslationZ, [0.3, 0.3, 1.0, 1.0], "Pos.Z"),
    (PropertyType::RotationX, [1.0, 0.6, 0.6, 1.0], "Rot.X"),
    (PropertyType::RotationY, [0.6, 1.0, 0.6, 1.0], "Rot.Y"),
    (PropertyType::RotationZ, [0.6, 0.6, 1.0, 1.0], "Rot.Z"),
    (PropertyType::ScaleX, [1.0, 0.8, 0.4, 1.0], "Scl.X"),
    (PropertyType::ScaleY, [0.8, 1.0, 0.4, 1.0], "Scl.Y"),
    (PropertyType::ScaleZ, [0.4, 0.8, 1.0, 1.0], "Scl.Z"),
];

struct ViewTransform {
    curve_origin: [f32; 2],
    curve_width: f32,
    curve_height: f32,
    duration: f32,
    val_range: f32,
    zoom_x: f32,
    zoom_y: f32,
    view_time_offset: f32,
    view_value_offset: f32,
}

impl ViewTransform {
    fn time_to_x(&self, time: f32) -> f32 {
        self.curve_origin[0]
            + (time - self.view_time_offset)
                / self.duration.max(0.001)
                * self.zoom_x
                * self.curve_width
    }

    fn value_to_y(&self, value: f32) -> f32 {
        self.curve_origin[1] + self.curve_height
            - (value - self.view_value_offset)
                / self.val_range.max(0.001)
                * self.zoom_y
                * self.curve_height
    }

    fn x_to_time(&self, x: f32) -> f32 {
        (x - self.curve_origin[0])
            / (self.zoom_x * self.curve_width).max(0.001)
            * self.duration.max(0.001)
            + self.view_time_offset
    }

    fn y_to_value(&self, y: f32) -> f32 {
        (self.curve_origin[1] + self.curve_height - y)
            / (self.zoom_y * self.curve_height).max(0.001)
            * self.val_range.max(0.001)
            + self.view_value_offset
    }
}

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
    pub visible_curves: HashSet<PropertyType>,
    pub window_size: [f32; 2],
    pub selected_keyframe: Option<SelectedKeyframe>,
    pub is_dragging_keyframe: bool,
    pub drag_start_mouse_pos: [f32; 2],
    pub zoom_x: f32,
    pub zoom_y: f32,
    pub view_time_offset: f32,
    pub view_value_offset: f32,
    pub view_val_range: f32,
    pub view_initialized: bool,
    pub is_scrubbing_ruler: bool,
    pub is_panning: bool,
    pub pan_start_mouse_pos: [f32; 2],
    pub pan_start_offset: [f32; 2],
}

impl Default for CurveEditorState {
    fn default() -> Self {
        let mut visible_curves = HashSet::new();
        visible_curves.insert(PropertyType::TranslationX);
        visible_curves.insert(PropertyType::TranslationY);
        visible_curves.insert(PropertyType::TranslationZ);
        visible_curves.insert(PropertyType::RotationX);
        visible_curves.insert(PropertyType::RotationY);
        visible_curves.insert(PropertyType::RotationZ);

        Self {
            is_open: false,
            selected_bone_id: None,
            visible_curves,
            window_size: [800.0, 500.0],
            selected_keyframe: None,
            is_dragging_keyframe: false,
            drag_start_mouse_pos: [0.0, 0.0],
            zoom_x: 1.0,
            zoom_y: 1.0,
            view_time_offset: 0.0,
            view_value_offset: 0.0,
            view_val_range: 2.0,
            view_initialized: false,
            is_scrubbing_ruler: false,
            is_panning: false,
            pan_start_mouse_pos: [0.0, 0.0],
            pan_start_offset: [0.0, 0.0],
        }
    }
}

pub fn build_curve_editor_window(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    timeline_state: &TimelineState,
    clip_library: &ClipLibrary,
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
        .size_constraints(
            [MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT],
            [display_size[0], display_size[1]],
        )
        .opened(&mut is_open)
        .build(|| {
            editor_state.window_size = ui.window_size();

            let content_region = ui.content_region_avail();

            ui.child_window("left_panel")
                .size([TRACK_LIST_WIDTH, content_region[1]])
                .border(true)
                .build(|| {
                    build_track_list(
                        ui,
                        timeline_state,
                        clip_library,
                        editor_state,
                    );
                });

            ui.same_line();

            let curve_view_width =
                content_region[0] - TRACK_LIST_WIDTH - 10.0;
            ui.child_window("curve_view")
                .size([curve_view_width, content_region[1]])
                .border(true)
                .build(|| {
                    build_curve_view(
                        ui,
                        ui_events,
                        timeline_state,
                        clip_library,
                        editor_state,
                    );
                });
        });

    editor_state.is_open = is_open;
}

fn build_track_list(
    ui: &imgui::Ui,
    timeline_state: &TimelineState,
    clip_library: &ClipLibrary,
    editor_state: &mut CurveEditorState,
) {
    let clip = match timeline_state
        .current_clip_id
        .and_then(|id| clip_library.get(id))
    {
        Some(c) => c,
        None => {
            ui.text("No clip selected");
            return;
        }
    };

    ui.text("Bones:");
    ui.separator();

    let mut sorted_bone_ids: Vec<BoneId> =
        clip.tracks.keys().copied().collect();
    sorted_bone_ids.sort();

    for bone_id in sorted_bone_ids {
        if let Some(track) = clip.tracks.get(&bone_id) {
            let is_selected =
                editor_state.selected_bone_id == Some(bone_id);
            let label = if track.bone_name.len() > 18 {
                format!("{}...", &track.bone_name[..15])
            } else {
                track.bone_name.clone()
            };

            if ui
                .selectable_config(&label)
                .selected(is_selected)
                .build()
            {
                editor_state.selected_bone_id = Some(bone_id);
                editor_state.view_initialized = false;
            }

            if is_selected {
                build_curve_selector_inline(ui, track, editor_state);
            }
        }
    }
}

fn build_curve_selector_inline(
    ui: &imgui::Ui,
    track: &crate::animation::editable::BoneTrack,
    editor_state: &mut CurveEditorState,
) {
    ui.indent();

    for (prop_type, color, name) in ALL_PROPERTY_TYPES {
        let curve = track.get_curve(*prop_type);
        if curve.is_empty() {
            continue;
        }

        let mut visible =
            editor_state.visible_curves.contains(prop_type);
        ui.text_colored(*color, "\u{25CF}");
        ui.same_line();
        if ui.checkbox(name, &mut visible) {
            if visible {
                editor_state.visible_curves.insert(*prop_type);
            } else {
                editor_state.visible_curves.remove(prop_type);
            }
        }
    }

    if ui.small_button("All") {
        for (prop_type, _, _) in ALL_PROPERTY_TYPES {
            editor_state.visible_curves.insert(*prop_type);
        }
    }
    ui.same_line();
    if ui.small_button("None") {
        editor_state.visible_curves.clear();
    }

    ui.unindent();
    ui.spacing();
}

fn build_curve_view(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    timeline_state: &TimelineState,
    clip_library: &ClipLibrary,
    editor_state: &mut CurveEditorState,
) {
    let clip = match timeline_state
        .current_clip_id
        .and_then(|id| clip_library.get(id))
    {
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
    let curve_area_width =
        content_region[0] - Y_AXIS_WIDTH - CURVE_PADDING * 2.0;
    let curve_area_height =
        content_region[1] - TIME_RULER_HEIGHT - CURVE_PADDING * 2.0;

    if curve_area_width <= 0.0 || curve_area_height <= 0.0 {
        return;
    }

    let mut curves_to_draw: Vec<(&PropertyCurve, [f32; 4], &str)> =
        Vec::new();
    for (prop_type, color, name) in ALL_PROPERTY_TYPES {
        if editor_state.visible_curves.contains(prop_type) {
            let curve = track.get_curve(*prop_type);
            if !curve.is_empty() {
                curves_to_draw.push((curve, *color, name));
            }
        }
    }

    let (global_min, global_max) =
        calculate_global_value_range(&curves_to_draw);

    if !editor_state.view_initialized {
        editor_state.view_value_offset = global_min;
        editor_state.view_val_range = global_max - global_min;
        editor_state.view_time_offset = 0.0;
        editor_state.zoom_x = 1.0;
        editor_state.zoom_y = 1.0;
        editor_state.view_initialized = true;
    }

    let val_range = editor_state.view_val_range;

    let draw_list = ui.get_window_draw_list();
    let cursor_pos = ui.cursor_screen_pos();

    let curve_origin = [
        cursor_pos[0] + Y_AXIS_WIDTH + CURVE_PADDING,
        cursor_pos[1] + TIME_RULER_HEIGHT + CURVE_PADDING,
    ];

    let vt = ViewTransform {
        curve_origin,
        curve_width: curve_area_width,
        curve_height: curve_area_height,
        duration: clip.duration,
        val_range,
        zoom_x: editor_state.zoom_x,
        zoom_y: editor_state.zoom_y,
        view_time_offset: editor_state.view_time_offset,
        view_value_offset: editor_state.view_value_offset,
    };

    let y_axis_origin = [
        cursor_pos[0],
        cursor_pos[1] + TIME_RULER_HEIGHT + CURVE_PADDING,
    ];
    draw_y_axis_labels(
        &draw_list,
        y_axis_origin,
        Y_AXIS_WIDTH,
        curve_area_height,
        &vt,
    );

    let ruler_pos = [
        cursor_pos[0] + Y_AXIS_WIDTH + CURVE_PADDING,
        cursor_pos[1],
    ];
    draw_time_ruler(
        &draw_list,
        ruler_pos,
        curve_area_width,
        &vt,
    );

    draw_list
        .add_rect(
            curve_origin,
            [
                curve_origin[0] + curve_area_width,
                curve_origin[1] + curve_area_height,
            ],
            [0.12, 0.12, 0.15, 1.0],
        )
        .filled(true)
        .build();

    draw_list.with_clip_rect_intersect(
        curve_origin,
        [
            curve_origin[0] + curve_area_width,
            curve_origin[1] + curve_area_height,
        ],
        || {
            draw_grid(
                &draw_list,
                curve_area_width,
                curve_area_height,
                &vt,
            );

            let sample_count =
                calculate_sample_count(curve_area_width);

            for (curve, color, _name) in &curves_to_draw {
                draw_curve_with_keyframes(
                    &draw_list,
                    curve,
                    *color,
                    sample_count,
                    &vt,
                );
            }

            if let Some(ref selected) =
                editor_state.selected_keyframe
            {
                draw_selected_keyframe_highlight(
                    &draw_list,
                    &curves_to_draw,
                    selected,
                    &vt,
                );
            }

            let playhead_x =
                vt.time_to_x(timeline_state.current_time);
            draw_list
                .add_line(
                    [playhead_x, curve_origin[1]],
                    [
                        playhead_x,
                        curve_origin[1] + curve_area_height,
                    ],
                    [1.0, 0.2, 0.2, 1.0],
                )
                .thickness(2.0)
                .build();

            if editor_state.is_dragging_keyframe {
                draw_keyframe_drag_preview(
                    &draw_list,
                    ui.io().mouse_pos,
                    &vt,
                );
            }
        },
    );

    let total_width =
        Y_AXIS_WIDTH + CURVE_PADDING + curve_area_width + CURVE_PADDING;
    let total_height = TIME_RULER_HEIGHT
        + CURVE_PADDING
        + curve_area_height
        + CURVE_PADDING;

    ui.set_cursor_screen_pos([cursor_pos[0], cursor_pos[1]]);
    ui.invisible_button(
        "curve_interaction_area",
        [total_width, total_height],
    );

    handle_mouse_interaction(
        ui,
        ui_events,
        editor_state,
        &vt,
        &curves_to_draw,
        ruler_pos,
        curve_area_width,
        clip.duration,
    );

    ui.set_cursor_screen_pos(
        [cursor_pos[0], cursor_pos[1] + total_height],
    );
}

fn handle_mouse_interaction(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    editor_state: &mut CurveEditorState,
    vt: &ViewTransform,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    ruler_pos: [f32; 2],
    curve_area_width: f32,
    duration: f32,
) {
    let is_hovered = ui.is_item_hovered();
    let mouse_pos = ui.io().mouse_pos;
    let mouse_clicked =
        ui.is_mouse_clicked(imgui::MouseButton::Left);
    let mouse_down = ui.io().mouse_down[0];
    let mouse_released =
        ui.is_mouse_released(imgui::MouseButton::Left);
    let middle_clicked =
        ui.is_mouse_clicked(imgui::MouseButton::Middle);
    let middle_released =
        ui.is_mouse_released(imgui::MouseButton::Middle);

    let in_ruler_area = mouse_pos[0] >= ruler_pos[0]
        && mouse_pos[0] <= ruler_pos[0] + curve_area_width
        && mouse_pos[1] >= ruler_pos[1]
        && mouse_pos[1] <= ruler_pos[1] + TIME_RULER_HEIGHT;

    let in_curve_area =
        mouse_pos[0] >= vt.curve_origin[0]
            && mouse_pos[0]
                <= vt.curve_origin[0] + vt.curve_width
            && mouse_pos[1] >= vt.curve_origin[1]
            && mouse_pos[1]
                <= vt.curve_origin[1] + vt.curve_height;

    handle_mouse_release(
        ui_events,
        editor_state,
        vt,
        mouse_pos,
        mouse_released,
        middle_released,
    );

    if is_hovered && mouse_clicked && in_ruler_area {
        editor_state.is_scrubbing_ruler = true;
        let time = vt.x_to_time(mouse_pos[0]).clamp(0.0, duration);
        ui_events.send(UIEvent::TimelineSetTime(time));
    }

    if is_hovered
        && mouse_clicked
        && in_curve_area
        && !editor_state.is_dragging_keyframe
        && !editor_state.is_panning
    {
        handle_curve_area_click(
            editor_state, mouse_pos, curves_to_draw, vt,
        );
    }

    if is_hovered && middle_clicked && in_curve_area {
        editor_state.is_panning = true;
        editor_state.pan_start_mouse_pos = mouse_pos;
        editor_state.pan_start_offset = [
            editor_state.view_time_offset,
            editor_state.view_value_offset,
        ];
    }

    if editor_state.is_panning && ui.io().mouse_down[2] {
        handle_panning(editor_state, mouse_pos, vt);
    }

    if editor_state.is_scrubbing_ruler && mouse_down {
        let time = vt.x_to_time(mouse_pos[0]).clamp(0.0, duration);
        ui_events.send(UIEvent::TimelineSetTime(time));
    }

    if is_hovered {
        handle_wheel_input(ui, editor_state, mouse_pos, vt);
    }
}

fn handle_mouse_release(
    ui_events: &mut UIEventQueue,
    editor_state: &mut CurveEditorState,
    vt: &ViewTransform,
    mouse_pos: [f32; 2],
    mouse_released: bool,
    middle_released: bool,
) {
    if mouse_released {
        if editor_state.is_dragging_keyframe {
            if let Some(ref selected) =
                editor_state.selected_keyframe
            {
                let new_time = vt
                    .x_to_time(mouse_pos[0])
                    .clamp(0.0, vt.duration);
                let new_value = vt.y_to_value(mouse_pos[1]);

                if let Some(bone_id) = editor_state.selected_bone_id
                {
                    ui_events.send(
                        UIEvent::TimelineMoveKeyframe {
                            bone_id,
                            property_type: selected
                                .property_type
                                .clone(),
                            keyframe_id: selected.keyframe_id,
                            new_time,
                            new_value,
                        },
                    );
                }
            }
        }
        editor_state.is_dragging_keyframe = false;
        editor_state.is_scrubbing_ruler = false;
    }

    if middle_released {
        editor_state.is_panning = false;
    }
}

fn handle_curve_area_click(
    editor_state: &mut CurveEditorState,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
) {
    let hit_keyframe =
        find_keyframe_at_position(mouse_pos, curves_to_draw, vt);

    if let Some((property_type, keyframe_id, time, value)) =
        hit_keyframe
    {
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

fn handle_panning(
    editor_state: &mut CurveEditorState,
    mouse_pos: [f32; 2],
    vt: &ViewTransform,
) {
    let dx = mouse_pos[0] - editor_state.pan_start_mouse_pos[0];
    let dy = mouse_pos[1] - editor_state.pan_start_mouse_pos[1];

    let time_per_pixel = vt.duration.max(0.001)
        / (vt.zoom_x * vt.curve_width).max(0.001);
    let value_per_pixel = vt.val_range.max(0.001)
        / (vt.zoom_y * vt.curve_height).max(0.001);

    editor_state.view_time_offset =
        editor_state.pan_start_offset[0] - dx * time_per_pixel;
    editor_state.view_value_offset =
        editor_state.pan_start_offset[1] + dy * value_per_pixel;
}

fn handle_wheel_input(
    ui: &imgui::Ui,
    editor_state: &mut CurveEditorState,
    mouse_pos: [f32; 2],
    vt: &ViewTransform,
) {
    let wheel = ui.io().mouse_wheel;
    if wheel == 0.0 {
        return;
    }

    let ctrl = ui.io().key_ctrl;

    if ctrl {
        zoom_at_mouse(editor_state, mouse_pos, wheel, vt);
    } else {
        let shift = ui.io().key_shift;
        pan_with_wheel(editor_state, wheel, shift, vt);
    }
}

fn zoom_at_mouse(
    editor_state: &mut CurveEditorState,
    mouse_pos: [f32; 2],
    wheel: f32,
    vt: &ViewTransform,
) {
    let mouse_time = vt.x_to_time(mouse_pos[0]);
    let mouse_value = vt.y_to_value(mouse_pos[1]);

    let factor = if wheel > 0.0 { 1.15 } else { 1.0 / 1.15 };
    let new_zoom_x =
        (editor_state.zoom_x * factor).clamp(0.1, 10.0);
    let new_zoom_y =
        (editor_state.zoom_y * factor).clamp(0.1, 10.0);

    editor_state.view_time_offset = mouse_time
        - (mouse_pos[0] - vt.curve_origin[0])
            / (new_zoom_x * vt.curve_width).max(0.001)
            * vt.duration.max(0.001);

    editor_state.view_value_offset = mouse_value
        - (vt.curve_origin[1] + vt.curve_height - mouse_pos[1])
            / (new_zoom_y * vt.curve_height).max(0.001)
            * vt.val_range.max(0.001);

    editor_state.zoom_x = new_zoom_x;
    editor_state.zoom_y = new_zoom_y;
}

fn pan_with_wheel(
    editor_state: &mut CurveEditorState,
    wheel: f32,
    shift: bool,
    vt: &ViewTransform,
) {
    if shift {
        let time_per_pixel = vt.duration.max(0.001)
            / (vt.zoom_x * vt.curve_width).max(0.001);
        editor_state.view_time_offset -=
            wheel * PAN_SPEED * time_per_pixel;
    } else {
        let value_per_pixel = vt.val_range.max(0.001)
            / (vt.zoom_y * vt.curve_height).max(0.001);
        editor_state.view_value_offset +=
            wheel * PAN_SPEED * value_per_pixel;
    }
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
    vt: &ViewTransform,
) {
    draw_list
        .add_rect(
            pos,
            [pos[0] + width, pos[1] + TIME_RULER_HEIGHT],
            [0.18, 0.18, 0.22, 1.0],
        )
        .filled(true)
        .build();

    let visible_duration =
        vt.duration / vt.zoom_x.max(0.001);
    let tick_interval = compute_nice_step(visible_duration / 8.0);

    let view_start = vt.view_time_offset;
    let view_end = view_start + visible_duration;

    let first_tick =
        (view_start / tick_interval).floor() * tick_interval;
    let mut time = first_tick;

    while time <= view_end + tick_interval {
        let x = vt.time_to_x(time);
        if x < pos[0] - 10.0 || x > pos[0] + width + 10.0 {
            time += tick_interval;
            continue;
        }

        let is_major =
            (time / tick_interval).round() as i32 % 4 == 0;
        let tick_height = if is_major { 10.0 } else { 5.0 };

        draw_list
            .add_line(
                [x, pos[1] + TIME_RULER_HEIGHT - tick_height],
                [x, pos[1] + TIME_RULER_HEIGHT],
                [0.6, 0.6, 0.6, 1.0],
            )
            .build();

        if is_major {
            draw_list.add_text(
                [x + 2.0, pos[1] + 2.0],
                [0.7, 0.7, 0.7, 1.0],
                &format!("{:.1}s", time),
            );
        }

        time += tick_interval;
    }
}

fn draw_y_axis_labels(
    draw_list: &imgui::DrawListMut,
    origin: [f32; 2],
    axis_width: f32,
    height: f32,
    vt: &ViewTransform,
) {
    let visible_range =
        vt.val_range / vt.zoom_y.max(0.001);
    if visible_range.abs() < 0.0001 {
        return;
    }

    let tick_count = calculate_y_tick_count(height);
    let raw_step = visible_range / tick_count as f32;
    let step = compute_nice_step(raw_step);

    let view_bottom = vt.view_value_offset;
    let view_top = view_bottom + visible_range;

    let first_tick = (view_bottom / step).ceil() * step;
    let label_color = [0.6, 0.6, 0.6, 1.0];
    let tick_color = [0.3, 0.3, 0.33, 0.5];

    let mut value = first_tick;
    while value <= view_top + step {
        let y = vt.value_to_y(value);
        if y < origin[1] - 10.0
            || y > origin[1] + height + 10.0
        {
            value += step;
            continue;
        }

        let label = format_value_label(value);
        draw_list.add_text(
            [origin[0] + 2.0, y - 7.0],
            label_color,
            &label,
        );

        draw_list
            .add_line(
                [origin[0] + axis_width - 4.0, y],
                [origin[0] + axis_width, y],
                tick_color,
            )
            .build();

        value += step;
    }
}

fn draw_grid(
    draw_list: &imgui::DrawListMut,
    width: f32,
    height: f32,
    vt: &ViewTransform,
) {
    let grid_color = [0.25, 0.25, 0.28, 1.0];

    let visible_duration =
        vt.duration / vt.zoom_x.max(0.001);
    let time_step = compute_nice_step(visible_duration / 8.0);
    let view_start_t = vt.view_time_offset;
    let view_end_t = view_start_t + visible_duration;
    let first_t = (view_start_t / time_step).floor() * time_step;

    let mut time = first_t;
    while time <= view_end_t + time_step {
        let x = vt.time_to_x(time);
        if x >= vt.curve_origin[0]
            && x <= vt.curve_origin[0] + width
        {
            draw_list
                .add_line(
                    [x, vt.curve_origin[1]],
                    [x, vt.curve_origin[1] + height],
                    grid_color,
                )
                .build();
        }
        time += time_step;
    }

    let visible_range =
        vt.val_range / vt.zoom_y.max(0.001);
    let value_step = compute_nice_step(visible_range / 6.0);
    let view_bottom = vt.view_value_offset;
    let view_top = view_bottom + visible_range;
    let first_v = (view_bottom / value_step).ceil() * value_step;

    let mut value = first_v;
    while value <= view_top + value_step {
        let y = vt.value_to_y(value);
        if y >= vt.curve_origin[1]
            && y <= vt.curve_origin[1] + height
        {
            let line_color = if value.abs() < value_step * 0.1 {
                [0.4, 0.4, 0.43, 1.0]
            } else {
                grid_color
            };

            draw_list
                .add_line(
                    [vt.curve_origin[0], y],
                    [vt.curve_origin[0] + width, y],
                    line_color,
                )
                .build();
        }
        value += value_step;
    }
}

fn draw_curve_with_keyframes(
    draw_list: &imgui::DrawListMut,
    curve: &PropertyCurve,
    color: [f32; 4],
    sample_count: usize,
    vt: &ViewTransform,
) {
    if curve.keyframes.is_empty() {
        return;
    }

    let step = vt.duration / sample_count as f32;
    let mut prev_point: Option<[f32; 2]> = None;

    for i in 0..=sample_count {
        let time = (i as f32) * step;
        if let Some(value) = curve.sample(time) {
            let point = [vt.time_to_x(time), vt.value_to_y(value)];

            if let Some(prev) = prev_point {
                draw_list
                    .add_line(prev, point, color)
                    .thickness(1.5)
                    .build();
            }

            prev_point = Some(point);
        }
    }

    for kf in &curve.keyframes {
        let x = vt.time_to_x(kf.time);
        let y = vt.value_to_y(kf.value);

        draw_list
            .add_circle([x, y], 5.0, color)
            .filled(true)
            .build();

        draw_list
            .add_circle([x, y], 5.0, [1.0, 1.0, 1.0, 0.8])
            .build();
    }
}

fn draw_selected_keyframe_highlight(
    draw_list: &imgui::DrawListMut,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    selected: &SelectedKeyframe,
    vt: &ViewTransform,
) {
    for (curve, _, _) in curves_to_draw {
        if curve.property_type == selected.property_type {
            if let Some(kf) =
                curve.get_keyframe(selected.keyframe_id)
            {
                let x = vt.time_to_x(kf.time);
                let y = vt.value_to_y(kf.value);

                draw_list
                    .add_circle(
                        [x, y],
                        8.0,
                        [1.0, 1.0, 0.0, 1.0],
                    )
                    .thickness(2.0)
                    .build();
            }
            break;
        }
    }
}

fn draw_keyframe_drag_preview(
    draw_list: &imgui::DrawListMut,
    mouse_pos: [f32; 2],
    vt: &ViewTransform,
) {
    let preview_x = mouse_pos[0].clamp(
        vt.curve_origin[0],
        vt.curve_origin[0] + vt.curve_width,
    );
    let preview_y = mouse_pos[1].clamp(
        vt.curve_origin[1],
        vt.curve_origin[1] + vt.curve_height,
    );

    draw_list
        .add_circle(
            [preview_x, preview_y],
            7.0,
            [1.0, 1.0, 0.0, 1.0],
        )
        .filled(true)
        .build();

    draw_list
        .add_circle(
            [preview_x, preview_y],
            7.0,
            [1.0, 1.0, 1.0, 1.0],
        )
        .thickness(2.0)
        .build();

    let preview_time = vt.x_to_time(preview_x);
    let preview_value = vt.y_to_value(preview_y);

    draw_list.add_text(
        [preview_x + 10.0, preview_y - 10.0],
        [1.0, 1.0, 1.0, 1.0],
        &format!("t={:.2}s v={:.3}", preview_time, preview_value),
    );
}

fn calculate_y_tick_count(height: f32) -> usize {
    ((height / 40.0) as usize).max(2).min(15)
}

fn compute_nice_step(raw_step: f32) -> f32 {
    if raw_step <= 0.0 {
        return 1.0;
    }
    let magnitude = 10.0f32.powf(raw_step.log10().floor());
    let normalized = raw_step / magnitude;

    let nice = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };

    nice * magnitude
}

fn format_value_label(value: f32) -> String {
    let abs = value.abs();
    if abs >= 100.0 {
        format!("{:.0}", value)
    } else if abs >= 1.0 {
        format!("{:.1}", value)
    } else {
        format!("{:.2}", value)
    }
}

fn calculate_global_value_range(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
) -> (f32, f32) {
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

fn find_keyframe_at_position(
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
                return Some((
                    curve.property_type.clone(),
                    kf.id,
                    kf.time,
                    kf.value,
                ));
            }
        }
    }

    None
}
