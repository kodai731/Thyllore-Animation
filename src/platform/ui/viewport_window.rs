use imgui::Condition;

use super::layout_snapshot::LayoutSnapshot;

#[derive(Clone, Debug, Default)]
pub struct ViewportInfo {
    pub size: [f32; 2],
    pub position: [f32; 2],
    pub focused: bool,
    pub hovered: bool,
}

pub fn build_viewport_window(
    ui: &imgui::Ui,
    texture_id: imgui::TextureId,
    current_size: [f32; 2],
    layout: &LayoutSnapshot,
) -> ViewportInfo {
    let mut info = ViewportInfo::default();

    ui.window("Scene")
        .position([layout.viewport_x, 0.0], Condition::Always)
        .size(
            [layout.viewport_width, layout.main_height],
            Condition::Always,
        )
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .bring_to_front_on_focus(false)
        .build(|| {
            let content_region = ui.content_region_avail();
            info.size = content_region;
            info.focused = ui.is_window_focused();
            info.hovered = ui.is_window_hovered();

            let window_pos = ui.window_pos();
            let cursor_pos = ui.cursor_pos();
            info.position = [window_pos[0] + cursor_pos[0], window_pos[1] + cursor_pos[1]];

            let display_size = if content_region[0] > 0.0 && content_region[1] > 0.0 {
                content_region
            } else {
                current_size
            };

            imgui::Image::new(texture_id, display_size).build(ui);
        });

    info
}
