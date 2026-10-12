use imgui::Condition;

use crate::asset::AssetStorage;
use crate::ecs::resource::{LayoutSnapshot, ViewportInput};
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

fn build_viewport_window(ui: &imgui::Ui, world: &World, _: &AssetStorage, _: &GraphicsResources) {
    let layout = world.resource::<LayoutSnapshot>();
    let mut viewport = world.resource_mut::<ViewportInput>();
    let texture_id = imgui::TextureId::new(viewport.texture_id);
    let image_size = [viewport.image_size[0] as f32, viewport.image_size[1] as f32];

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
            viewport.size = content_region;
            viewport.focused = ui.is_window_focused();
            viewport.hovered = ui.is_window_hovered();

            let window_pos = ui.window_pos();
            let cursor_pos = ui.cursor_pos();
            viewport.position = [window_pos[0] + cursor_pos[0], window_pos[1] + cursor_pos[1]];

            let display_size = if content_region[0] > 0.0 && content_region[1] > 0.0 {
                content_region
            } else {
                image_size
            };
            imgui::Image::new(ui, texture_id, display_size).build();
        });

    let new_width = viewport.size[0] as u32;
    let new_height = viewport.size[1] as u32;
    if new_width > 0 && new_height > 0 && [new_width, new_height] != viewport.image_size {
        viewport.resize_pending = Some((new_width, new_height));
    }
}

crate::ui_window!("viewport", Viewport, 0, build_viewport_window);
