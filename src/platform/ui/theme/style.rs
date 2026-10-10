use imgui::{Style, StyleColor};

use super::colors::*;
use crate::ecs::resource::{density_spacing, UiSettings};

const HEADER_ALPHA: f32 = 0.18;
const HEADER_HOVERED_ALPHA: f32 = 0.25;
const HEADER_ACTIVE_ALPHA: f32 = 0.32;

pub fn apply_thyllore_style(style: &mut Style) {
    style.window_rounding = 10.0;
    style.child_rounding = 10.0;
    style.popup_rounding = 10.0;
    style.frame_rounding = 6.0;
    style.grab_rounding = 6.0;
    style.tab_rounding = 6.0;
    style.scrollbar_rounding = 9.0;
    style.scrollbar_size = 10.0;
    style.window_padding = [12.0, 12.0];
    style.frame_padding = [10.0, 5.0];
    style.item_spacing = [8.0, 6.0];
    style.item_inner_spacing = [6.0, 4.0];
    style.window_border_size = 0.0;
    style.frame_border_size = 0.0;
    style.tab_border_size = 0.0;
    style.circle_tesselation_max_error = 0.15;
    style.anti_aliased_lines = true;
    style.anti_aliased_fill = true;

    let c = &mut style.colors;

    c[StyleColor::Text as usize] = srgb_to_linear(TEXT);
    c[StyleColor::TextDisabled as usize] = srgb_to_linear(TEXT_SECONDARY);
    c[StyleColor::WindowBg as usize] = srgb_to_linear(SURFACE0);
    c[StyleColor::ChildBg as usize] = srgb_to_linear(SURFACE1);
    c[StyleColor::PopupBg as usize] = srgb_to_linear(SURFACE1);
    c[StyleColor::MenuBarBg as usize] = srgb_to_linear(SURFACE1);
    c[StyleColor::Border as usize] = srgb_to_linear(OUTLINE);
    c[StyleColor::FrameBg as usize] = srgb_to_linear(SURFACE2);
    c[StyleColor::FrameBgHovered as usize] = srgb_to_linear(SURFACE3);
    c[StyleColor::Button as usize] = srgb_to_linear(SURFACE2);
    c[StyleColor::ButtonHovered as usize] = srgb_to_linear(SURFACE3);
    c[StyleColor::ButtonActive as usize] = srgb_to_linear(ACCENT);
    c[StyleColor::CheckMark as usize] = srgb_to_linear(ACCENT);
    c[StyleColor::SliderGrab as usize] = srgb_to_linear(ACCENT);
    c[StyleColor::SliderGrabActive as usize] = srgb_to_linear(ACCENT);
    c[StyleColor::TabActive as usize] = srgb_to_linear(ACCENT);
    c[StyleColor::TextSelectedBg as usize] = [
        srgb_to_linear(ACCENT)[0],
        srgb_to_linear(ACCENT)[1],
        srgb_to_linear(ACCENT)[2],
        0.25,
    ];

    c[StyleColor::Header as usize] = [
        srgb_to_linear(ACCENT)[0],
        srgb_to_linear(ACCENT)[1],
        srgb_to_linear(ACCENT)[2],
        HEADER_ALPHA,
    ];
    c[StyleColor::HeaderHovered as usize] = [
        srgb_to_linear(ACCENT)[0],
        srgb_to_linear(ACCENT)[1],
        srgb_to_linear(ACCENT)[2],
        HEADER_HOVERED_ALPHA,
    ];
    c[StyleColor::HeaderActive as usize] = [
        srgb_to_linear(ACCENT)[0],
        srgb_to_linear(ACCENT)[1],
        srgb_to_linear(ACCENT)[2],
        HEADER_ACTIVE_ALPHA,
    ];

    c[StyleColor::TitleBg as usize] = srgb_to_linear(SURFACE0);
    c[StyleColor::TitleBgActive as usize] = srgb_to_linear(SURFACE0);
    c[StyleColor::TitleBgCollapsed as usize] = srgb_to_linear(SURFACE0);

    c[StyleColor::Tab as usize] = srgb_to_linear(SURFACE1);
    c[StyleColor::TabHovered as usize] = srgb_to_linear(SURFACE3);

    c[StyleColor::ScrollbarBg as usize] = srgb_to_linear(SURFACE1);
    c[StyleColor::ScrollbarGrab as usize] = srgb_to_linear(SURFACE2);
    c[StyleColor::ScrollbarGrabHovered as usize] = srgb_to_linear(SURFACE3);
    c[StyleColor::ScrollbarGrabActive as usize] = srgb_to_linear(ACCENT);

    c[StyleColor::Separator as usize] = srgb_to_linear(OUTLINE);
    c[StyleColor::SeparatorHovered as usize] = srgb_to_linear(SURFACE3);
    c[StyleColor::SeparatorActive as usize] = srgb_to_linear(ACCENT);

    c[StyleColor::ResizeGrip as usize] = srgb_to_linear(OUTLINE);
    c[StyleColor::ResizeGripHovered as usize] = srgb_to_linear(SURFACE3);
    c[StyleColor::ResizeGripActive as usize] = srgb_to_linear(ACCENT);

    c[StyleColor::TabUnfocused as usize] = srgb_to_linear(SURFACE1);
    c[StyleColor::TabUnfocusedActive as usize] = srgb_to_linear(SURFACE2);

    c[StyleColor::ModalWindowDimBg as usize] = [0.0, 0.0, 0.0, 0.5];

    c[StyleColor::NavHighlight as usize] = srgb_to_linear(ACCENT);
    c[StyleColor::NavWindowingHighlight as usize] = [0.0, 0.0, 0.0, 0.7];
    c[StyleColor::NavWindowingDimBg as usize] = [0.0, 0.0, 0.0, 0.5];

    c[StyleColor::DragDropTarget as usize] = [
        srgb_to_linear(ACCENT)[0],
        srgb_to_linear(ACCENT)[1],
        srgb_to_linear(ACCENT)[2],
        0.9,
    ];

    c[StyleColor::PlotLines as usize] = srgb_to_linear(TEXT_SECONDARY);
    c[StyleColor::PlotLinesHovered as usize] = srgb_to_linear(ACCENT);
    c[StyleColor::PlotHistogram as usize] = srgb_to_linear(TEXT_SECONDARY);
    c[StyleColor::PlotHistogramHovered as usize] = srgb_to_linear(ACCENT);

    c[StyleColor::TableHeaderBg as usize] = srgb_to_linear(SURFACE2);
    c[StyleColor::TableBorderStrong as usize] = srgb_to_linear(OUTLINE);
    c[StyleColor::TableBorderLight as usize] = [
        srgb_to_linear(OUTLINE)[0],
        srgb_to_linear(OUTLINE)[1],
        srgb_to_linear(OUTLINE)[2],
        srgb_to_linear(OUTLINE)[3] * 0.5,
    ];
    c[StyleColor::TableRowBg as usize] = srgb_to_linear(SURFACE1);
    c[StyleColor::TableRowBgAlt as usize] = srgb_to_linear(SURFACE0);

    c[StyleColor::BorderShadow as usize] = [0.0, 0.0, 0.0, 0.0];

    c[StyleColor::DockingPreview as usize] = [
        srgb_to_linear(ACCENT)[0],
        srgb_to_linear(ACCENT)[1],
        srgb_to_linear(ACCENT)[2],
        0.7,
    ];
    c[StyleColor::DockingEmptyBg as usize] = srgb_to_linear(SURFACE0);
}

pub fn apply_ui_settings(context: &mut imgui::Context, settings: &UiSettings) {
    context.io_mut().font_global_scale = settings.scale;

    let spacing = density_spacing(settings.density);
    let style = context.style_mut();
    style.frame_padding = spacing.frame_padding;
    style.item_spacing = spacing.item_spacing;
}
