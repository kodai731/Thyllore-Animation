use imgui::Condition;

use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::HierarchyState;
use crate::ecs::systems::query_hierarchy_tree;
use crate::ecs::world::World;

pub fn build_hierarchy_window(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    world: &World,
    state: &HierarchyState,
) {
    let display_size = ui.io().display_size;
    let hierarchy_width = 250.0;
    let debug_height = 250.0;
    let timeline_height = 300.0;
    let main_height = display_size[1] - debug_height - timeline_height;

    ui.window("Hierarchy")
        .position([0.0, 0.0], Condition::Always)
        .size([hierarchy_width, main_height], Condition::Always)
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .build(|| {
            build_search_bar(ui, ui_events, state);
            ui.separator();
            build_entity_tree(ui, ui_events, world, state);
        });
}

fn build_search_bar(ui: &imgui::Ui, ui_events: &mut UIEventQueue, state: &HierarchyState) {
    let mut search_text = state.search_filter.clone();
    ui.set_next_item_width(-1.0);
    if ui
        .input_text("##search", &mut search_text)
        .hint("Search...")
        .build()
    {
        ui_events.send(UIEvent::SetSearchFilter(search_text));
    }
}

fn build_entity_tree(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    world: &World,
    state: &HierarchyState,
) {
    let entries = query_hierarchy_tree(world, state);

    for entry in entries {
        let indent = entry.depth as f32 * 16.0;
        let cursor_pos = ui.cursor_pos();
        ui.set_cursor_pos([cursor_pos[0] + indent, cursor_pos[1]]);

        let expand_button_width = 16.0;

        if entry.has_children {
            let expand_symbol = if entry.expanded { "v" } else { ">" };
            if ui.small_button(&format!("{}##{}", expand_symbol, entry.entity)) {
                if entry.expanded {
                    ui_events.send(UIEvent::CollapseEntity(entry.entity));
                } else {
                    ui_events.send(UIEvent::ExpandEntity(entry.entity));
                }
            }
            ui.same_line();
        } else {
            let cursor = ui.cursor_pos();
            ui.set_cursor_pos([cursor[0] + expand_button_width, cursor[1]]);
        }

        let icon_label = format!("[{}]", entry.icon_char);
        ui.text(&icon_label);
        ui.same_line();

        let label = format!("{}##{}", entry.name, entry.entity);
        let selected = entry.selected;

        if ui.selectable_config(&label).selected(selected).build() {
            if ui.io().key_ctrl {
                ui_events.send(UIEvent::ToggleEntitySelection(entry.entity));
            } else {
                ui_events.send(UIEvent::SelectEntity(entry.entity));
            }
        }

        if ui.is_item_hovered() && ui.is_mouse_double_clicked(imgui::MouseButton::Left) {
            ui_events.send(UIEvent::FocusOnEntity(entry.entity));
        }
    }
}
