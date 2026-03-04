use crate::animation::editable::SourceClipId;
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{ClipLibrary, PoseLibrary};

pub fn build_pose_library_panel(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    pose_library: &mut PoseLibrary,
    clip_library: &ClipLibrary,
) {
    if !imgui::CollapsingHeader::new("Poses")
        .default_open(true)
        .build(ui)
    {
        return;
    }

    build_pose_toolbar(ui, ui_events, pose_library);

    if pose_library.naming_active {
        build_name_input(ui, ui_events, pose_library);
    }

    ui.separator();
    build_pose_list(ui, pose_library, clip_library);
}

fn build_pose_toolbar(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    pose_library: &mut PoseLibrary,
) {
    if ui.small_button("Save Pose") {
        pose_library.naming_active = true;
        pose_library.name_buffer = format!("Pose {}", pose_library.pose_ids.len() + 1);
    }

    ui.same_line();
    let has_selection = pose_library.selected_pose_id.is_some();

    if has_selection {
        if ui.small_button("Apply") {
            if let Some(id) = pose_library.selected_pose_id {
                ui_events.send(UIEvent::PoseLibraryApply(id));
            }
        }
    } else {
        ui.text_disabled("Apply");
    }

    ui.same_line();
    if has_selection {
        if ui.small_button("Del##pose") {
            if let Some(id) = pose_library.selected_pose_id {
                ui_events.send(UIEvent::PoseLibraryDelete(id));
            }
        }
    } else {
        ui.text_disabled("Del");
    }
}

fn build_name_input(ui: &imgui::Ui, ui_events: &mut UIEventQueue, pose_library: &mut PoseLibrary) {
    ui.set_next_item_width(-1.0);
    let entered = ui
        .input_text("##pose_name", &mut pose_library.name_buffer)
        .enter_returns_true(true)
        .hint("Pose name...")
        .build();

    if entered && !pose_library.name_buffer.is_empty() {
        let name = pose_library.name_buffer.clone();
        ui_events.send(UIEvent::PoseLibrarySaveCurrent { name });
        pose_library.naming_active = false;
        pose_library.name_buffer.clear();
    }
}

fn build_pose_list(ui: &imgui::Ui, pose_library: &mut PoseLibrary, clip_library: &ClipLibrary) {
    if pose_library.pose_ids.is_empty() {
        ui.text_disabled("No saved poses");
        return;
    }

    let pose_ids: Vec<SourceClipId> = pose_library.pose_ids.clone();

    for &pose_id in &pose_ids {
        let name = clip_library
            .get(pose_id)
            .map(|c| c.name.as_str())
            .unwrap_or("(unknown)");

        let label = format!("{}##pose_{}", name, pose_id);
        let is_selected = pose_library.selected_pose_id == Some(pose_id);

        if ui.selectable_config(&label).selected(is_selected).build() {
            pose_library.selected_pose_id = Some(pose_id);
        }
    }
}
