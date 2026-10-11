use imgui::{Style, StyleColor};

use super::colors::*;
use crate::ecs::resource::{density_spacing, UiSettings};

const HEADER_ALPHA: f32 = 0.18;
const HEADER_HOVERED_ALPHA: f32 = 0.25;
const HEADER_ACTIVE_ALPHA: f32 = 0.32;

pub fn apply_thyllore_style(style: &mut Style) {
    style.set_window_rounding(10.0);
    style.set_child_rounding(10.0);
    style.set_popup_rounding(10.0);
    style.set_frame_rounding(6.0);
    style.set_grab_rounding(6.0);
    style.set_tab_rounding(6.0);
    style.set_scrollbar_rounding(9.0);
    style.set_scrollbar_size(10.0);
    style.set_window_padding([12.0, 12.0]);
    style.set_frame_padding([10.0, 5.0]);
    style.set_item_spacing([8.0, 6.0]);
    style.set_item_inner_spacing([6.0, 4.0]);
    style.set_window_border_size(0.0);
    style.set_frame_border_size(0.0);
    style.set_tab_border_size(0.0);
    style.set_circle_tessellation_max_error(0.15);
    style.set_anti_aliased_lines(true);
    style.set_anti_aliased_fill(true);

    style.set_color(StyleColor::Text, srgb_to_linear(TEXT));
    style.set_color(StyleColor::TextDisabled, srgb_to_linear(TEXT_SECONDARY));
    style.set_color(StyleColor::WindowBg, srgb_to_linear(SURFACE0));
    style.set_color(StyleColor::ChildBg, srgb_to_linear(SURFACE1));
    style.set_color(StyleColor::PopupBg, srgb_to_linear(SURFACE1));
    style.set_color(StyleColor::MenuBarBg, srgb_to_linear(SURFACE1));
    style.set_color(StyleColor::Border, srgb_to_linear(OUTLINE));
    style.set_color(StyleColor::FrameBg, srgb_to_linear(SURFACE2));
    style.set_color(StyleColor::FrameBgHovered, srgb_to_linear(SURFACE3));
    style.set_color(StyleColor::Button, srgb_to_linear(SURFACE2));
    style.set_color(StyleColor::ButtonHovered, srgb_to_linear(SURFACE3));
    style.set_color(StyleColor::ButtonActive, srgb_to_linear(ACCENT));
    style.set_color(StyleColor::CheckMark, srgb_to_linear(ACCENT));
    style.set_color(StyleColor::SliderGrab, srgb_to_linear(ACCENT));
    style.set_color(StyleColor::SliderGrabActive, srgb_to_linear(ACCENT));
    style.set_color(StyleColor::TabSelected, srgb_to_linear(ACCENT));
    style.set_color(
        StyleColor::TextSelectedBg,
        [
            srgb_to_linear(ACCENT)[0],
            srgb_to_linear(ACCENT)[1],
            srgb_to_linear(ACCENT)[2],
            0.25,
        ],
    );

    style.set_color(
        StyleColor::Header,
        [
            srgb_to_linear(ACCENT)[0],
            srgb_to_linear(ACCENT)[1],
            srgb_to_linear(ACCENT)[2],
            HEADER_ALPHA,
        ],
    );
    style.set_color(
        StyleColor::HeaderHovered,
        [
            srgb_to_linear(ACCENT)[0],
            srgb_to_linear(ACCENT)[1],
            srgb_to_linear(ACCENT)[2],
            HEADER_HOVERED_ALPHA,
        ],
    );
    style.set_color(
        StyleColor::HeaderActive,
        [
            srgb_to_linear(ACCENT)[0],
            srgb_to_linear(ACCENT)[1],
            srgb_to_linear(ACCENT)[2],
            HEADER_ACTIVE_ALPHA,
        ],
    );

    style.set_color(StyleColor::TitleBg, srgb_to_linear(SURFACE0));
    style.set_color(StyleColor::TitleBgActive, srgb_to_linear(SURFACE0));
    style.set_color(StyleColor::TitleBgCollapsed, srgb_to_linear(SURFACE0));

    style.set_color(StyleColor::Tab, srgb_to_linear(SURFACE1));
    style.set_color(StyleColor::TabHovered, srgb_to_linear(SURFACE3));

    style.set_color(StyleColor::ScrollbarBg, srgb_to_linear(SURFACE1));
    style.set_color(StyleColor::ScrollbarGrab, srgb_to_linear(SURFACE2));
    style.set_color(StyleColor::ScrollbarGrabHovered, srgb_to_linear(SURFACE3));
    style.set_color(StyleColor::ScrollbarGrabActive, srgb_to_linear(ACCENT));

    style.set_color(StyleColor::Separator, srgb_to_linear(OUTLINE));
    style.set_color(StyleColor::SeparatorHovered, srgb_to_linear(SURFACE3));
    style.set_color(StyleColor::SeparatorActive, srgb_to_linear(ACCENT));

    style.set_color(StyleColor::ResizeGrip, srgb_to_linear(OUTLINE));
    style.set_color(StyleColor::ResizeGripHovered, srgb_to_linear(SURFACE3));
    style.set_color(StyleColor::ResizeGripActive, srgb_to_linear(ACCENT));

    style.set_color(StyleColor::TabDimmed, srgb_to_linear(SURFACE1));
    style.set_color(StyleColor::TabDimmedSelected, srgb_to_linear(SURFACE2));

    style.set_color(StyleColor::ModalWindowDimBg, [0.0, 0.0, 0.0, 0.5]);

    style.set_color(StyleColor::NavCursor, srgb_to_linear(ACCENT));
    style.set_color(StyleColor::NavWindowingHighlight, [0.0, 0.0, 0.0, 0.7]);
    style.set_color(StyleColor::NavWindowingDimBg, [0.0, 0.0, 0.0, 0.5]);

    style.set_color(
        StyleColor::DragDropTarget,
        [
            srgb_to_linear(ACCENT)[0],
            srgb_to_linear(ACCENT)[1],
            srgb_to_linear(ACCENT)[2],
            0.9,
        ],
    );

    style.set_color(StyleColor::PlotLines, srgb_to_linear(TEXT_SECONDARY));
    style.set_color(StyleColor::PlotLinesHovered, srgb_to_linear(ACCENT));
    style.set_color(StyleColor::PlotHistogram, srgb_to_linear(TEXT_SECONDARY));
    style.set_color(StyleColor::PlotHistogramHovered, srgb_to_linear(ACCENT));

    style.set_color(StyleColor::TableHeaderBg, srgb_to_linear(SURFACE2));
    style.set_color(StyleColor::TableBorderStrong, srgb_to_linear(OUTLINE));
    style.set_color(
        StyleColor::TableBorderLight,
        [
            srgb_to_linear(OUTLINE)[0],
            srgb_to_linear(OUTLINE)[1],
            srgb_to_linear(OUTLINE)[2],
            srgb_to_linear(OUTLINE)[3] * 0.5,
        ],
    );
    style.set_color(StyleColor::TableRowBg, srgb_to_linear(SURFACE1));
    style.set_color(StyleColor::TableRowBgAlt, srgb_to_linear(SURFACE0));

    style.set_color(StyleColor::BorderShadow, [0.0, 0.0, 0.0, 0.0]);

    style.set_color(
        StyleColor::DockingPreview,
        [
            srgb_to_linear(ACCENT)[0],
            srgb_to_linear(ACCENT)[1],
            srgb_to_linear(ACCENT)[2],
            0.7,
        ],
    );
    style.set_color(StyleColor::DockingEmptyBg, srgb_to_linear(SURFACE0));
}

pub fn apply_ui_settings(context: &mut imgui::Context, settings: &UiSettings) {
    let spacing = density_spacing(settings.density);
    let style = context.style_mut();
    style.set_font_scale_main(settings.scale);
    style.set_frame_padding(spacing.frame_padding);
    style.set_item_spacing(spacing.item_spacing);
}
