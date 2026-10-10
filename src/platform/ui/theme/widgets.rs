use imgui::Ui;

use crate::ecs::resource::UiWidgetState;
use crate::ecs::World;

use super::anim::ui_anim;
use super::colors::{srgb_to_linear, ACCENT, SURFACE1, SURFACE3, TEXT};

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
    draw_list.add_text(text_pos, srgb_to_linear(TEXT), display_label);

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
}
