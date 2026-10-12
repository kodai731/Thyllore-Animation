use imgui::Condition;

use crate::asset::AssetStorage;
use crate::ecs::resource::MessageLog;
use crate::ecs::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[cfg(debug_assertions)]
use super::debug_window::build_debug_panel_content;
use super::message_window::build_message_window_content;
use crate::ecs::resource::LayoutSnapshot;

fn draw_bottom_panel(
    ui: &imgui::Ui,
    ecs_world: &World,
    message_log: &mut MessageLog,
    layout: &LayoutSnapshot,
) {
    ui.window("Bottom Panel")
        .position([0.0, layout.debug_y], Condition::Always)
        .size(
            [layout.display_size[0], layout.debug_height],
            Condition::Always,
        )
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .bring_to_front_on_focus(false)
        .build(|| draw_bottom_tabs(ui, ecs_world, message_log));
}

fn draw_bottom_tabs(ui: &imgui::Ui, ecs_world: &World, message_log: &mut MessageLog) {
    let Some(_tab_bar) = ui.tab_bar("bottom_tabs") else {
        return;
    };

    draw_messages_tab(ui, ecs_world, message_log);
    #[cfg(debug_assertions)]
    draw_debug_tab(ui, ecs_world);
}

fn draw_messages_tab(ui: &imgui::Ui, ecs_world: &World, message_log: &mut MessageLog) {
    let label = build_message_tab_label(message_log);
    let flags = if message_log.select_tab_requested {
        imgui::TabItemFlags::SET_SELECTED
    } else {
        imgui::TabItemFlags::empty()
    };
    message_log.select_tab_requested = false;

    let Some(_tab) = ui.tab_item_with_flags(&label, None, flags) else {
        return;
    };
    ui.child_window("messages_content").build(ui, || {
        build_message_window_content(ui, ecs_world, message_log)
    });
}

#[cfg(debug_assertions)]
fn draw_debug_tab(ui: &imgui::Ui, ecs_world: &World) {
    let Some(_tab) = ui.tab_item("Debug") else {
        return;
    };
    ui.child_window("debug_content")
        .build(ui, || build_debug_panel_content(ui, ecs_world));
}

fn build_message_tab_label(message_log: &MessageLog) -> String {
    if message_log.error_count > 0 {
        format!("Messages ({} errors)###messages", message_log.error_count)
    } else if message_log.warning_count > 0 {
        format!(
            "Messages ({} warnings)###messages",
            message_log.warning_count
        )
    } else {
        "Messages###messages".to_string()
    }
}

fn build_bottom_panel(ui: &imgui::Ui, world: &World, _: &AssetStorage, _: &GraphicsResources) {
    let mut message_log = world.resource_mut::<MessageLog>();
    message_log.sync_from_buffer();
    let layout = world.resource::<LayoutSnapshot>();
    draw_bottom_panel(ui, world, &mut message_log, &layout);
}

crate::ui_window!("bottom_panel", Side, 0, build_bottom_panel);
