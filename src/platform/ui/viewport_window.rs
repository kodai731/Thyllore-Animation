use imgui::Condition;

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
) -> ViewportInfo {
    let mut info = ViewportInfo::default();

    let display_size = ui.io().display_size;
    let hierarchy_width = 250.0;
    let inspector_width = 300.0;
    let debug_height = 250.0;
    let main_height = display_size[1] - debug_height;
    let viewport_width = display_size[0] - hierarchy_width - inspector_width;
    let viewport_x = hierarchy_width;

    ui.window("Scene")
        .position([viewport_x, 0.0], Condition::Always)
        .size([viewport_width, main_height], Condition::Always)
        .resizable(false)
        .movable(false)
        .collapsible(false)
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
