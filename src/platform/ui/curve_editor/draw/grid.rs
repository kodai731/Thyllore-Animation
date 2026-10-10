use crate::ecs::resource::PoseLibrary;

use super::super::view::{
    calculate_y_tick_count, compute_nice_step, format_value_label, ViewTransform,
};
use super::super::window::TIME_RULER_HEIGHT;

pub(in crate::platform::ui::curve_editor) fn draw_time_ruler(
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

    let visible_duration = vt.duration / vt.zoom_x.max(0.001);
    let tick_interval = compute_nice_step(visible_duration / 8.0);

    let view_start = vt.view_time_offset;
    let view_end = view_start + visible_duration;

    let first_tick = (view_start / tick_interval).floor() * tick_interval;
    let mut time = first_tick;

    while time <= view_end + tick_interval {
        let x = vt.time_to_x(time);
        if x < pos[0] - 10.0 || x > pos[0] + width + 10.0 {
            time += tick_interval;
            continue;
        }

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
            draw_list.add_text(
                [x + 2.0, pos[1] + 2.0],
                [0.7, 0.7, 0.7, 1.0],
                &format!("{:.1}s", time),
            );
        }

        time += tick_interval;
    }
}

pub(in crate::platform::ui::curve_editor) fn draw_y_axis_labels(
    draw_list: &imgui::DrawListMut,
    origin: [f32; 2],
    axis_width: f32,
    height: f32,
    vt: &ViewTransform,
) {
    let visible_range = vt.val_range / vt.zoom_y.max(0.001);
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
        if y < origin[1] - 10.0 || y > origin[1] + height + 10.0 {
            value += step;
            continue;
        }

        let label = format_value_label(value);
        draw_list.add_text([origin[0] + 2.0, y - 7.0], label_color, &label);

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

pub(in crate::platform::ui::curve_editor) fn draw_pose_markers(
    draw_list: &imgui::DrawListMut,
    vt: &ViewTransform,
    curve_area_height: f32,
    pose_library: &PoseLibrary,
) {
    let unselected_color = [0.8, 0.7, 0.2, 0.35];
    let selected_color = [1.0, 0.85, 0.0, 0.9];

    for entry in &pose_library.poses {
        let x = vt.time_to_x(entry.captured_time);
        let is_selected = pose_library.selected_pose_id == Some(entry.id);
        let top = vt.curve_origin[1];
        let bottom = top + curve_area_height;

        if is_selected {
            draw_list
                .add_line([x, top], [x, bottom], selected_color)
                .thickness(2.0)
                .build();
        } else {
            let dash_len = 6.0;
            let gap_len = 4.0;
            let mut y = top;
            while y < bottom {
                let y_end = (y + dash_len).min(bottom);
                draw_list
                    .add_line([x, y], [x, y_end], unselected_color)
                    .thickness(1.0)
                    .build();
                y += dash_len + gap_len;
            }
        }

        let diamond_size = 5.0;
        let diamond_y = top + 8.0;
        let color = if is_selected {
            selected_color
        } else {
            unselected_color
        };
        let top_pt = [x, diamond_y - diamond_size];
        let right_pt = [x + diamond_size, diamond_y];
        let bottom_pt = [x, diamond_y + diamond_size];
        let left_pt = [x - diamond_size, diamond_y];
        if is_selected {
            draw_list
                .add_triangle(top_pt, right_pt, bottom_pt, color)
                .filled(true)
                .build();
            draw_list
                .add_triangle(top_pt, bottom_pt, left_pt, color)
                .filled(true)
                .build();
        } else {
            draw_list.add_line(top_pt, right_pt, color).build();
            draw_list.add_line(right_pt, bottom_pt, color).build();
            draw_list.add_line(bottom_pt, left_pt, color).build();
            draw_list.add_line(left_pt, top_pt, color).build();
        }
    }
}

pub(in crate::platform::ui::curve_editor) fn draw_grid(
    draw_list: &imgui::DrawListMut,
    width: f32,
    height: f32,
    vt: &ViewTransform,
) {
    let grid_color = [0.25, 0.25, 0.28, 1.0];

    let visible_duration = vt.duration / vt.zoom_x.max(0.001);
    let time_step = compute_nice_step(visible_duration / 8.0);
    let view_start_t = vt.view_time_offset;
    let view_end_t = view_start_t + visible_duration;
    let first_t = (view_start_t / time_step).floor() * time_step;

    let mut time = first_t;
    while time <= view_end_t + time_step {
        let x = vt.time_to_x(time);
        if x >= vt.curve_origin[0] && x <= vt.curve_origin[0] + width {
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

    let visible_range = vt.val_range / vt.zoom_y.max(0.001);
    let value_step = compute_nice_step(visible_range / 6.0);
    let view_bottom = vt.view_value_offset;
    let view_top = view_bottom + visible_range;
    let first_v = (view_bottom / value_step).ceil() * value_step;

    let mut value = first_v;
    while value <= view_top + value_step {
        let y = vt.value_to_y(value);
        if y >= vt.curve_origin[1] && y <= vt.curve_origin[1] + height {
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
