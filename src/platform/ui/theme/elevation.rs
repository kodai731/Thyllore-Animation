use imgui::{ColorStackToken, StyleColor, Ui};

use super::colors::{srgb_to_linear_rgba, EDGE_SHADOW, SURFACE1, SURFACE2};
use super::shadow::draw_window_shadow;

const EDGE_STROKE: f32 = 1.0;

/// Fields drop in declaration order, which must be the reverse of the push order.
#[must_use]
pub struct RaisedWindowStyle<'ui> {
    _title_active: ColorStackToken<'ui>,
    _title: ColorStackToken<'ui>,
    _window_bg: ColorStackToken<'ui>,
}

/// Tonal elevation: a window that floats over the panels is one surface step lighter than them.
pub fn push_raised_window_style(ui: &Ui) -> RaisedWindowStyle<'_> {
    let window_bg = ui.push_style_color(StyleColor::WindowBg, srgb_to_linear_rgba(SURFACE1));
    let title = ui.push_style_color(StyleColor::TitleBg, srgb_to_linear_rgba(SURFACE1));
    let title_active =
        ui.push_style_color(StyleColor::TitleBgActive, srgb_to_linear_rgba(SURFACE2));
    RaisedWindowStyle {
        _title_active: title_active,
        _title: title,
        _window_bg: window_bg,
    }
}

/// Call first inside the window's build closure: layered shadow plus the dark outer hairline.
pub fn draw_raised_window_frame(ui: &Ui) {
    let rounding = ui.clone_style().window_rounding();
    draw_window_shadow(ui, rounding);
    draw_window_edge(ui, rounding);
}

/// The dark half of the dual-tone edge; imgui's own border draws the light inner half.
pub fn draw_window_edge(ui: &Ui, rounding: f32) {
    let draw_list = ui.get_window_draw_list();
    let [x, y] = ui.window_pos();
    let [width, height] = ui.window_size();
    let outer_min = [x - EDGE_STROKE, y - EDGE_STROKE];
    let outer_max = [x + width + EDGE_STROKE, y + height + EDGE_STROKE];

    draw_list.with_clip_rect(outer_min, outer_max, || {
        draw_list
            .add_rect(outer_min, outer_max, srgb_to_linear_rgba(EDGE_SHADOW))
            .rounding(rounding + EDGE_STROKE)
            .thickness(EDGE_STROKE)
            .build();
    });
}
