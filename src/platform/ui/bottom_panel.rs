use imgui::Condition;

use crate::app::GUIData;
use crate::ecs::events::UIEventQueue;
use crate::ecs::resource::MessageLog;
use crate::ecs::World;

#[cfg(debug_assertions)]
use super::debug_window::{build_debug_panel_content, DebugWindowState};
use super::layout_snapshot::LayoutSnapshot;
use super::message_window::build_message_window_content;

pub fn build_bottom_panel(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    #[cfg(debug_assertions)] debug_state: &mut DebugWindowState,
    gui_data: &mut GUIData,
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
        .build(|| {
            let msg_tab_label = build_message_tab_label(message_log);

            imgui::TabBar::new("bottom_tabs").build(ui, || {
                #[cfg(debug_assertions)]
                imgui::TabItem::new("Debug").build(ui, || {
                    build_debug_panel_content(ui, ui_events, debug_state, gui_data, ecs_world);
                });

                imgui::TabItem::new(&msg_tab_label).build(ui, || {
                    build_message_window_content(ui, message_log);
                });
            });
        });
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
