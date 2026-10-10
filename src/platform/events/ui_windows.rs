use crate::app::App;
use crate::ecs::resource::{LayoutSnapshot, PanelLayout, ViewportInput};
use crate::hooks::ui_window::UiWindows;
use crate::platform::ui::pointer::release_idle_ui_pointer;

/// Draws every registered window in panel order; the windows read `LayoutSnapshot` and `ViewportInput`.
pub(super) fn build_ui_windows(ui: &imgui::Ui, app: &mut App) {
    publish_layout_snapshot(app, ui.io().display_size);
    publish_viewport_image(app);
    release_idle_ui_pointer(ui, &app.data.ecs_world);

    let windows = app.data.ecs_world.resource::<UiWindows>().ordered();
    for window in windows {
        (window.build)(
            ui,
            &app.data.ecs_world,
            &app.data.ecs_assets,
            &app.data.graphics_resources,
        );
    }
}

fn publish_layout_snapshot(app: &mut App, display_size: [f32; 2]) {
    let snapshot = {
        let mut panel_layout = app.data.ecs_world.resource_mut::<PanelLayout>();
        panel_layout.constrain_to_display(display_size[0], display_size[1]);
        LayoutSnapshot::from_layout(&panel_layout, display_size)
    };
    app.data.ecs_world.insert_resource(snapshot);
}

fn publish_viewport_image(app: &App) {
    let mut viewport = app.data.ecs_world.resource_mut::<ViewportInput>();
    viewport.texture_id = app.data.viewport.texture_id();
    viewport.image_size = [app.data.viewport.width, app.data.viewport.height];
}
