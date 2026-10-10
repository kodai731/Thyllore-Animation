use std::borrow::Cow;

use imgui::{DrawListMut, StyleVar, Ui};

use crate::ecs::resource::UiWidgetState;
use crate::ecs::World;
use crate::platform::ui::pointer::{is_last_item_double_clicked, read_ui_pointer};

use super::anim::ui_anim;
use super::colors::{
    srgb_to_linear, ACCENT, AXIS_X, AXIS_Y, AXIS_Z, OUTLINE, SURFACE1, SURFACE2, SURFACE3, TEXT,
    TEXT_SECONDARY,
};
use super::fonts::UiFonts;
use super::icons::Icon;

const TREE_INDENT: f32 = 16.0;
pub const ICON_BUTTON_SIZE: f32 = 24.0;
pub const PROPERTY_LABEL_WIDTH: f32 = 96.0;
const TOOLBAR_DIVIDER_WIDTH: f32 = 12.0;
const TOOLBAR_DIVIDER_HEIGHT: f32 = 18.0;
const SEGMENT_PADDING_X: f32 = 8.0;
const SEGMENT_ROUNDING: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonState {
    Normal,
    Active,
}

pub fn icon_button(ui: &Ui, icon: Icon, tooltip: &str, state: ButtonState) -> bool {
    let size = ICON_BUTTON_SIZE;
    let glyph = icon.glyph();
    let text_size = ui.calc_text_size(&glyph);
    let id = ui.new_id_str(tooltip).raw();

    let clicked = ui.invisible_button(format!("icon_btn_{}", id), [size, size]);
    let hovered = ui.is_item_hovered();

    if hovered {
        ui.tooltip_text(tooltip);
    }

    let draw_list = ui.get_window_draw_list();
    let rect_min = ui.item_rect_min();
    let rect_max = ui.item_rect_max();

    let bg_color = match state {
        ButtonState::Active => srgb_to_linear([ACCENT[0], ACCENT[1], ACCENT[2], 0.3]),
        ButtonState::Normal if hovered => srgb_to_linear(SURFACE3),
        ButtonState::Normal => [0.0, 0.0, 0.0, 0.0],
    };

    draw_list
        .add_rect(rect_min, rect_max, bg_color)
        .rounding(6.0)
        .filled(true)
        .build();

    let icon_color = match state {
        ButtonState::Active => srgb_to_linear(ACCENT),
        ButtonState::Normal => srgb_to_linear(TEXT),
    };

    let center_x = rect_min[0] + (size - text_size[0]) * 0.5;
    let center_y = rect_min[1] + (size - text_size[1]) * 0.5;
    draw_list.add_text([center_x, center_y], icon_color, &glyph);

    clicked
}

const ELLIPSIS: char = '…';
const AXIS_LABEL_GAP: f32 = 2.0;

pub fn truncate_label(text: &str, max_chars: usize) -> Cow<'_, str> {
    if text.chars().count() <= max_chars {
        return Cow::Borrowed(text);
    }

    let mut truncated: String = text.chars().take(max_chars).collect();
    truncated.push(ELLIPSIS);
    Cow::Owned(truncated)
}

pub fn property_label(ui: &Ui, label: &str) {
    let display = fit_label_to_column(ui, label);
    ui.text_colored(srgb_to_linear(TEXT_SECONDARY), &display);
    if matches!(display, Cow::Owned(_)) && ui.is_item_hovered() {
        ui.tooltip_text(label);
    }

    ui.same_line_with_pos(PROPERTY_LABEL_WIDTH);
    ui.set_next_item_width(-1.0);
}

fn fit_label_to_column<'a>(ui: &Ui, label: &'a str) -> Cow<'a, str> {
    let fits = |candidate: &str| ui.calc_text_size(candidate)[0] <= PROPERTY_LABEL_WIDTH;
    if fits(label) {
        return Cow::Borrowed(label);
    }

    let fitting_chars = (0..label.chars().count())
        .rev()
        .find(|&max_chars| fits(&truncate_label(label, max_chars)))
        .unwrap_or(0);
    truncate_label(label, fitting_chars)
}

pub fn vector3_field(ui: &Ui, id: &str, values: &mut [f32; 3], speed: f32, format: &str) -> bool {
    let component_width = vector3_component_width(ui.content_region_avail()[0]);
    let axes = [("X", AXIS_X, "x"), ("Y", AXIS_Y, "y"), ("Z", AXIS_Z, "z")];

    let mut changed = false;
    for (index, (axis_label, axis_color, suffix)) in axes.into_iter().enumerate() {
        if index > 0 {
            ui.same_line_with_spacing(0.0, 0.0);
        }
        ui.text_colored(srgb_to_linear(axis_color), axis_label);
        ui.same_line_with_spacing(0.0, AXIS_LABEL_GAP);

        let label_width = ui.calc_text_size(axis_label)[0] + AXIS_LABEL_GAP;
        ui.set_next_item_width(component_width - label_width);
        changed |= imgui::Drag::new(format!("##{id}_{suffix}"))
            .speed(speed)
            .display_format(format)
            .build(ui, &mut values[index]);
    }
    changed
}

pub fn vector3_component_width(available: f32) -> f32 {
    available / 3.0
}

pub fn toolbar_divider(ui: &Ui) {
    ui.same_line_with_spacing(0.0, 0.0);
    let [x, y] = ui.cursor_screen_pos();
    let line_x = x + TOOLBAR_DIVIDER_WIDTH * 0.5;
    let line_top = y + (ICON_BUTTON_SIZE - TOOLBAR_DIVIDER_HEIGHT) * 0.5;
    ui.get_window_draw_list()
        .add_line(
            [line_x, line_top],
            [line_x, line_top + TOOLBAR_DIVIDER_HEIGHT],
            srgb_to_linear(OUTLINE),
        )
        .thickness(1.0)
        .build();

    ui.dummy([TOOLBAR_DIVIDER_WIDTH, ICON_BUTTON_SIZE]);
    ui.same_line_with_spacing(0.0, 0.0);
}

fn segment_width(text_size: [f32; 2], padding: f32) -> f32 {
    text_size[0] + 2.0 * padding
}

pub fn segmented_control(ui: &Ui, id: &str, options: &[&str], selected: usize) -> Option<usize> {
    let _id_token = ui.push_id(id);
    let last_index = options.len().saturating_sub(1);
    let mut clicked = None;

    for (i, option) in options.iter().enumerate() {
        if i > 0 {
            ui.same_line_with_spacing(0.0, 0.0);
        }

        let text_size = ui.calc_text_size(option);
        let width = segment_width(text_size, SEGMENT_PADDING_X);
        if ui.invisible_button(option, [width, ICON_BUTTON_SIZE]) {
            clicked = Some(i);
        }
        let hovered = ui.is_item_hovered();
        let rect_min = ui.item_rect_min();
        let rect_max = ui.item_rect_max();

        let fill = if i == selected {
            ACCENT
        } else if hovered {
            SURFACE3
        } else {
            SURFACE2
        };
        let is_first = i == 0;
        let is_last = i == last_index;
        let rounding = if is_first || is_last {
            SEGMENT_ROUNDING
        } else {
            0.0
        };

        let draw_list = ui.get_window_draw_list();
        draw_list
            .add_rect(rect_min, rect_max, srgb_to_linear(fill))
            .rounding(rounding)
            .round_top_left(is_first)
            .round_bot_left(is_first)
            .round_top_right(is_last)
            .round_bot_right(is_last)
            .filled(true)
            .build();
        let text_pos = [
            rect_min[0] + SEGMENT_PADDING_X,
            rect_min[1] + (ICON_BUTTON_SIZE - text_size[1]) * 0.5,
        ];
        draw_list.add_text(text_pos, srgb_to_linear(TEXT), option);
    }

    clicked
}

#[derive(Clone, Copy, Debug)]
pub enum SectionDefault {
    Open,
    Closed,
}

fn strip_display_suffix(label: &str) -> &str {
    if let Some(idx) = label.find("##") {
        &label[..idx]
    } else {
        label
    }
}

fn triangle_points(center: [f32; 2], size: f32, angle_rad: f32) -> [[f32; 2]; 3] {
    let half = size * 0.5;
    let direction = [angle_rad.cos(), angle_rad.sin()];
    let perpendicular = [-direction[1], direction[0]];

    let tip = [
        center[0] + half * direction[0],
        center[1] + half * direction[1],
    ];
    let base_left = [
        center[0] - half * perpendicular[0],
        center[1] - half * perpendicular[1],
    ];
    let base_right = [
        center[0] + half * perpendicular[0],
        center[1] + half * perpendicular[1],
    ];

    [tip, base_left, base_right]
}

fn toggle_knob_x(left: f32, width: f32, knob_radius: f32, t: f32) -> f32 {
    let track_inner_width = width - 2.0 * knob_radius;
    left + knob_radius + track_inner_width * t
}

pub fn section_header(ui: &Ui, world: &World, label: &str, default: SectionDefault) -> bool {
    let display_label = strip_display_suffix(label);
    let id = ui.new_id_str(label).raw();
    let default_open = matches!(default, SectionDefault::Open);

    let row_height = ui.frame_height();
    let row_width = ui.content_region_avail()[0];
    let clicked = ui.invisible_button(format!("section_{}", id), [row_width, row_height]);
    let hovered = ui.is_item_hovered();
    let rect_min = ui.item_rect_min();
    let rect_max = ui.item_rect_max();

    let is_open = match world.get_resource_mut::<UiWidgetState>() {
        Some(mut state) => {
            let open = state.section_open.entry(id).or_insert(default_open);
            if clicked {
                *open = !*open;
            }
            *open
        }
        None => default_open,
    };

    let target = if is_open { 1.0 } else { 0.0 };
    let t = ui_anim(ui, world, &format!("section_angle_{}", id), target);
    let angle_rad = t * std::f32::consts::FRAC_PI_2;

    let draw_list = ui.get_window_draw_list();
    let bg_color = if hovered {
        srgb_to_linear(SURFACE3)
    } else {
        srgb_to_linear(SURFACE1)
    };
    draw_list
        .add_rect(rect_min, rect_max, bg_color)
        .rounding(6.0)
        .filled(true)
        .build();

    let frame_padding = ui.clone_style().frame_padding;
    let font_height = ui.text_line_height();
    let triangle_size = font_height * 0.5;
    let triangle_center = [
        rect_min[0] + frame_padding[0] + triangle_size * 0.5,
        rect_min[1] + row_height * 0.5,
    ];
    let points = triangle_points(triangle_center, triangle_size, angle_rad);
    draw_list
        .add_triangle(points[0], points[1], points[2], srgb_to_linear(TEXT))
        .filled(true)
        .build();

    let text_pos = [
        rect_min[0] + frame_padding[0] * 2.0 + triangle_size,
        rect_min[1] + frame_padding[1],
    ];
    if let Some(fonts) = world.get_resource::<UiFonts>() {
        let _font_stack = ui.push_font(fonts.heading);
        draw_list.add_text(text_pos, srgb_to_linear(TEXT), display_label);
    } else {
        draw_list.add_text(text_pos, srgb_to_linear(TEXT), display_label);
    }

    is_open
}

pub fn toggle_switch(ui: &Ui, world: &World, label: &str, value: &mut bool) -> bool {
    let id = ui.new_id_str(label).raw();
    let display_label = strip_display_suffix(label);

    let pill_width = 28.0;
    let pill_height = 16.0;
    let knob_radius = 5.0;

    let label_width = ui.calc_text_size(display_label)[0];
    let spacing = ui.clone_style().item_spacing[0];
    let font_height = ui.text_line_height();
    let row_height = font_height.max(pill_height);
    let total_width = pill_width + spacing + label_width;

    let clicked = ui.invisible_button(format!("toggle_btn_{}", id), [total_width, row_height]);
    let origin = ui.item_rect_min();
    if clicked {
        *value = !*value;
    }

    let target = if *value { 1.0 } else { 0.0 };
    let t = ui_anim(ui, world, &format!("toggle_{}", id), target);

    let draw_list = ui.get_window_draw_list();
    let pill_left = origin[0];
    let pill_top = origin[1] + (row_height - pill_height) * 0.5;
    let pill_color = if *value {
        srgb_to_linear(ACCENT)
    } else {
        srgb_to_linear(SURFACE3)
    };
    draw_list
        .add_rect(
            [pill_left, pill_top],
            [pill_left + pill_width, pill_top + pill_height],
            pill_color,
        )
        .rounding(pill_height * 0.5)
        .filled(true)
        .build();

    let knob_center = [
        toggle_knob_x(pill_left, pill_width, knob_radius, t),
        pill_top + pill_height * 0.5,
    ];
    draw_list
        .add_circle(knob_center, knob_radius, [1.0, 1.0, 1.0, 1.0])
        .filled(true)
        .build();

    let text_pos = [
        origin[0] + pill_width + spacing,
        origin[1] + (row_height - font_height) * 0.5,
    ];
    draw_list.add_text(text_pos, srgb_to_linear(TEXT), display_label);

    clicked
}

fn draw_search_icon(draw_list: &DrawListMut, center: [f32; 2], color: [f32; 4]) {
    let radius = 4.0;
    draw_list
        .add_circle(center, radius, color)
        .thickness(1.5)
        .build();

    let handle_start = [center[0] + radius * 0.7, center[1] + radius * 0.7];
    let handle_end = [center[0] + radius * 1.6, center[1] + radius * 1.6];
    draw_list
        .add_line(handle_start, handle_end, color)
        .thickness(1.5)
        .build();
}

fn draw_clear_cross(
    draw_list: &DrawListMut,
    rect_min: [f32; 2],
    rect_max: [f32; 2],
    color: [f32; 4],
) {
    let center = [
        (rect_min[0] + rect_max[0]) * 0.5,
        (rect_min[1] + rect_max[1]) * 0.5,
    ];
    let half = (rect_max[1] - rect_min[1]) * 0.2;
    draw_list
        .add_line(
            [center[0] - half, center[1] - half],
            [center[0] + half, center[1] + half],
            color,
        )
        .thickness(1.5)
        .build();
    draw_list
        .add_line(
            [center[0] - half, center[1] + half],
            [center[0] + half, center[1] - half],
            color,
        )
        .thickness(1.5)
        .build();
}

pub fn search_field(ui: &Ui, id: &str, hint: &str, text: &mut String) -> bool {
    let icon_gutter = 22.0;
    let style = ui.clone_style();
    let field_height = ui.frame_height();
    let available_width = ui.content_region_avail()[0];
    let input_width = if text.is_empty() {
        available_width
    } else {
        available_width - field_height - style.item_spacing[0]
    };

    ui.set_next_item_width(input_width);
    let mut changed = {
        let _padding = ui.push_style_var(StyleVar::FramePadding([
            icon_gutter,
            style.frame_padding[1],
        ]));
        ui.input_text(format!("##{}", id), text).hint(hint).build()
    };

    let draw_list = ui.get_window_draw_list();
    let field_min = ui.item_rect_min();
    let icon_center = [
        field_min[0] + icon_gutter * 0.5,
        field_min[1] + field_height * 0.5,
    ];
    draw_search_icon(&draw_list, icon_center, srgb_to_linear(TEXT_SECONDARY));

    if !text.is_empty() {
        ui.same_line();
        if ui.invisible_button(format!("{}_clear", id), [field_height, field_height]) {
            text.clear();
            changed = true;
        }
        let cross_color = if ui.is_item_hovered() {
            srgb_to_linear(TEXT)
        } else {
            srgb_to_linear(TEXT_SECONDARY)
        };
        draw_clear_cross(
            &draw_list,
            ui.item_rect_min(),
            ui.item_rect_max(),
            cross_color,
        );
    }

    changed
}

pub fn is_in_expand_hit(mouse_x: f32, row_left: f32, depth: usize) -> bool {
    let expand_left = row_left + depth as f32 * TREE_INDENT;
    mouse_x >= expand_left && mouse_x < expand_left + TREE_INDENT
}

#[derive(Clone)]
pub struct TreeRowSpec<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub icon: &'a str,
    pub depth: usize,
    pub has_children: bool,
    pub expanded: bool,
    pub selected: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TreeRowResponse {
    None,
    Clicked,
    ExpandToggled,
    DoubleClicked,
}

pub fn tree_row(ui: &Ui, world: &World, spec: &TreeRowSpec) -> TreeRowResponse {
    let id = ui.new_id_str(spec.id).raw();
    let row_height = ui.frame_height();
    let row_width = ui.content_region_avail()[0];

    let clicked = ui.invisible_button(format!("tree_row_{}", id), [row_width, row_height]);
    let hovered = ui.is_item_hovered();
    let double_clicked = is_last_item_double_clicked(ui);
    let rect_min = ui.item_rect_min();
    let rect_max = ui.item_rect_max();
    let row_left = rect_min[0];

    let pointer = read_ui_pointer(ui);
    let on_expand_triangle =
        spec.has_children && is_in_expand_hit(pointer.pos[0], row_left, spec.depth);
    let response = match (on_expand_triangle, clicked, double_clicked) {
        (true, true, _) => TreeRowResponse::ExpandToggled,
        (true, false, _) => TreeRowResponse::None,
        (false, _, true) => TreeRowResponse::DoubleClicked,
        (false, true, false) => TreeRowResponse::Clicked,
        (false, false, false) => TreeRowResponse::None,
    };

    let draw_list = ui.get_window_draw_list();

    if hovered {
        draw_list
            .add_rect(rect_min, rect_max, srgb_to_linear(SURFACE3))
            .rounding(6.0)
            .filled(true)
            .build();
    }

    if spec.selected {
        let accent_alpha = [ACCENT[0], ACCENT[1], ACCENT[2], 0.25];
        draw_list
            .add_rect(rect_min, rect_max, srgb_to_linear(accent_alpha))
            .rounding(6.0)
            .filled(true)
            .build();

        let bar_left = rect_min[0];
        let bar_right = bar_left + 2.0;
        draw_list
            .add_rect(
                [bar_left, rect_min[1]],
                [bar_right, rect_max[1]],
                srgb_to_linear(ACCENT),
            )
            .filled(true)
            .build();
    }

    let indent_x = row_left + spec.depth as f32 * TREE_INDENT;
    if spec.has_children {
        let target = if spec.expanded { 1.0 } else { 0.0 };
        let t = ui_anim(ui, world, &format!("tree_expand_{}", id), target);
        let angle_rad = t * std::f32::consts::FRAC_PI_2;

        let triangle_center = [indent_x + TREE_INDENT * 0.5, rect_min[1] + row_height * 0.5];
        let triangle_size = ui.text_line_height() * 0.5;
        let points = triangle_points(triangle_center, triangle_size, angle_rad);
        draw_list
            .add_triangle(points[0], points[1], points[2], srgb_to_linear(TEXT))
            .filled(true)
            .build();
    }

    let text_y = rect_min[1] + ui.clone_style().frame_padding[1];
    let icon_x = indent_x + TREE_INDENT;
    draw_list.add_text([icon_x, text_y], srgb_to_linear(TEXT), spec.icon);

    let label_x = icon_x + ui.calc_text_size(spec.icon)[0] + 4.0;
    draw_list.add_text([label_x, text_y], srgb_to_linear(TEXT), spec.label);

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triangle_points_angle_zero() {
        let center = [10.0, 20.0];
        let size = 8.0;
        let points = triangle_points(center, size, 0.0);

        assert!((points[0][0] - 14.0).abs() < 1e-6, "tip x at 0°");
        assert!((points[0][1] - 20.0).abs() < 1e-6, "tip y at 0°");
        assert!((points[1][0] - 10.0).abs() < 1e-6, "base left x at 0°");
        assert!((points[1][1] - 16.0).abs() < 1e-6, "base left y at 0°");
        assert!((points[2][0] - 10.0).abs() < 1e-6, "base right x at 0°");
        assert!((points[2][1] - 24.0).abs() < 1e-6, "base right y at 0°");
    }

    #[test]
    fn test_triangle_points_angle_90_degrees() {
        let center = [10.0, 20.0];
        let size = 8.0;
        let points = triangle_points(center, size, std::f32::consts::FRAC_PI_2);

        assert!((points[0][0] - 10.0).abs() < 1e-6, "tip x at 90°");
        assert!((points[0][1] - 24.0).abs() < 1e-6, "tip y at 90°");
        assert!((points[1][0] - 14.0).abs() < 1e-6, "base left x at 90°");
        assert!((points[1][1] - 20.0).abs() < 1e-6, "base left y at 90°");
        assert!((points[2][0] - 6.0).abs() < 1e-6, "base right x at 90°");
        assert!((points[2][1] - 20.0).abs() < 1e-6, "base right y at 90°");
    }

    #[test]
    fn test_toggle_knob_x_t_zero() {
        let left = 0.0;
        let width = 28.0;
        let knob_radius = 5.0;
        let x = toggle_knob_x(left, width, knob_radius, 0.0);
        assert!(
            (x - 5.0).abs() < 1e-6,
            "knob at t=0 should be at left + radius"
        );
    }

    #[test]
    fn test_toggle_knob_x_t_one() {
        let left = 0.0;
        let width = 28.0;
        let knob_radius = 5.0;
        let x = toggle_knob_x(left, width, knob_radius, 1.0);
        assert!(
            (x - 23.0).abs() < 1e-6,
            "knob at t=1 should be at left + width - radius"
        );
    }

    #[test]
    fn test_strip_display_suffix_no_hash() {
        assert_eq!(strip_display_suffix("Hello"), "Hello");
    }

    #[test]
    fn test_strip_display_suffix_with_hash() {
        assert_eq!(strip_display_suffix("Hello##12345"), "Hello");
    }

    #[test]
    fn test_is_in_expand_hit_depth_0() {
        let row_left = 0.0;
        assert!(!is_in_expand_hit(-0.1, row_left, 0));
        assert!(is_in_expand_hit(0.0, row_left, 0));
        assert!(is_in_expand_hit(15.9, row_left, 0));
        assert!(!is_in_expand_hit(16.0, row_left, 0));
    }

    #[test]
    fn test_is_in_expand_hit_depth_1() {
        let row_left = 0.0;
        assert!(!is_in_expand_hit(15.9, row_left, 1));
        assert!(is_in_expand_hit(16.0, row_left, 1));
        assert!(is_in_expand_hit(31.9, row_left, 1));
        assert!(!is_in_expand_hit(32.0, row_left, 1));
    }

    #[test]
    fn test_is_in_expand_hit_with_offset_row_left() {
        let row_left = 10.0;
        assert!(!is_in_expand_hit(9.9, row_left, 0));
        assert!(is_in_expand_hit(10.0, row_left, 0));
        assert!(!is_in_expand_hit(26.0, row_left, 0));
    }

    #[test]
    fn test_segment_width() {
        let text_size = [50.0, 14.0];
        let width = segment_width(text_size, 8.0);
        assert!(
            (width - 66.0).abs() < 1e-6,
            "width = text_width + 2*padding"
        );
    }

    #[test]
    fn test_segment_width_zero_text() {
        let text_size = [0.0, 14.0];
        let width = segment_width(text_size, 8.0);
        assert!((width - 16.0).abs() < 1e-6, "empty text still has padding");
    }

    #[test]
    fn test_truncate_label_short() {
        let result = truncate_label("Hello", 10);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_truncate_label_exact() {
        let result = truncate_label("Hello", 5);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_truncate_label_long() {
        let result = truncate_label("Hello World", 5);
        assert_eq!(result, "Hello…");
    }

    #[test]
    fn test_truncate_label_empty() {
        let result = truncate_label("", 5);
        assert_eq!(result, "");
    }

    #[test]
    fn test_vector3_component_width() {
        assert!((vector3_component_width(300.0) - 100.0).abs() < 1e-6);
        assert!((vector3_component_width(96.0) - 32.0).abs() < 1e-6);
    }
}
