use imgui::Condition;

use crate::asset::AssetStorage;
use crate::ecs::events::{send_dialog_request, ClipExportFormat, DialogRequest};
use crate::ecs::resource::{ClipBrowserState, ClipLibrary, GltfModelCache, TimelineState};
use crate::ecs::systems::phases::event_dispatch::clip_browser::ClipBrowserEvent;
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::world::World;
use thyllore_anim_core::editable::{EditableAnimationClip, SourceClipId};
use thyllore_vulkan_core::resource::GraphicsResources;

use crate::ecs::resource::LayoutSnapshot;

fn draw_clip_browser_window(
    ui: &imgui::Ui,
    clip_library: &ClipLibrary,
    browser_state: &mut ClipBrowserState,
    timeline_state: &TimelineState,
    world: &World,
    layout: &LayoutSnapshot,
) {
    ui.window("Clip Browser")
        .position([0.0, layout.clip_browser_y], Condition::Always)
        .size(
            [layout.hierarchy_width, layout.clip_browser_height],
            Condition::Always,
        )
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .build(|| {
            let reference_counts = crate::ecs::systems::clip_library_count_references(world);

            build_toolbar(ui, timeline_state, &reference_counts, world);
            ui.separator();
            build_filter_bar(ui, browser_state);
            ui.separator();
            build_clip_list(
                ui,
                clip_library,
                browser_state,
                timeline_state,
                &reference_counts,
                world,
            );
        });
}

fn clip_action_button<Action>(
    ui: &imgui::Ui,
    label: &str,
    available_action: Option<Action>,
) -> Option<Action> {
    let Some(action) = available_action else {
        ui.text_disabled(label);
        return None;
    };

    ui.small_button(label).then_some(action)
}

fn count_clip_references(
    reference_counts: &[(SourceClipId, usize)],
    clip_id: SourceClipId,
) -> usize {
    reference_counts
        .iter()
        .find(|(counted_id, _)| *counted_id == clip_id)
        .map_or(0, |(_, count)| *count)
}

/// Keeps the next button on the current row while it fits, otherwise lets it start a new row.
fn continue_toolbar_row(ui: &imgui::Ui, next_label: &str, toolbar_right_edge: f32) {
    let style = ui.clone_style();
    let next_button_width = ui.calc_text_size(next_label)[0] + style.frame_padding()[0] * 2.0;
    let next_button_right = ui.item_rect_max()[0] + style.item_spacing()[0] + next_button_width;

    if next_button_right <= toolbar_right_edge {
        ui.same_line();
    }
}

fn build_toolbar(
    ui: &imgui::Ui,
    timeline_state: &TimelineState,
    reference_counts: &[(SourceClipId, usize)],
    world: &World,
) {
    let toolbar_right_edge = ui.cursor_screen_pos()[0] + ui.content_region_avail()[0];

    if ui.small_button("+ New") {
        world.send_command(ClipBrowserEvent::CreateEmpty);
    }

    continue_toolbar_row(ui, "Load", toolbar_right_edge);
    if ui.small_button("Load") {
        send_dialog_request(world, DialogRequest::LoadClip);
    }

    let selected_clip = timeline_state.current_clip_id;
    let clip_export = |format: ClipExportFormat| {
        selected_clip.map(|source_id| DialogRequest::ExportClip { source_id, format })
    };

    continue_toolbar_row(ui, "Save", toolbar_right_edge);
    if let Some(source_id) = clip_action_button(ui, "Save", selected_clip) {
        send_dialog_request(world, DialogRequest::SaveClip(source_id));
    }

    continue_toolbar_row(ui, "FBX", toolbar_right_edge);
    if let Some(request) = clip_action_button(ui, "FBX", clip_export(ClipExportFormat::Fbx)) {
        send_dialog_request(world, request);
    }

    continue_toolbar_row(ui, "glTF", toolbar_right_edge);
    let has_model = world
        .get_resource::<GltfModelCache>()
        .map_or(false, |c| c.has_model());
    let gltf_export = clip_export(ClipExportFormat::Gltf)
        .or_else(|| has_model.then_some(DialogRequest::ExportModelGltf));
    if let Some(request) = clip_action_button(ui, "glTF", gltf_export) {
        send_dialog_request(world, request);
    }

    continue_toolbar_row(ui, "glTF (anim only)", toolbar_right_edge);
    let animation_only_export = clip_export(ClipExportFormat::GltfAnimationOnly);
    if let Some(request) = clip_action_button(ui, "glTF (anim only)", animation_only_export) {
        send_dialog_request(world, request);
    }

    continue_toolbar_row(ui, "Dup", toolbar_right_edge);
    if let Some(source_id) = clip_action_button(ui, "Dup", selected_clip) {
        world.send_command(ClipBrowserEvent::Duplicate(source_id));
    }

    continue_toolbar_row(ui, "Del", toolbar_right_edge);
    let deletable_clip =
        selected_clip.filter(|clip_id| count_clip_references(reference_counts, *clip_id) == 0);
    if let Some(source_id) = clip_action_button(ui, "Del", deletable_clip) {
        world.send_command(ClipBrowserEvent::Delete(source_id));
    }
}

fn build_filter_bar(ui: &imgui::Ui, browser_state: &mut ClipBrowserState) {
    ui.set_next_item_width(-1.0);
    ui.input_text("##clip_filter", &mut browser_state.filter_text)
        .hint("Filter...")
        .build();
}

fn build_clip_list(
    ui: &imgui::Ui,
    clip_library: &ClipLibrary,
    browser_state: &mut ClipBrowserState,
    timeline_state: &TimelineState,
    reference_counts: &[(SourceClipId, usize)],
    world: &World,
) {
    let clip_names = crate::ecs::systems::clip_library_clip_names(clip_library);

    if clip_names.is_empty() {
        ui.text_disabled("No clips");
        return;
    }

    let filter_lower = browser_state.filter_text.to_lowercase();

    let remaining = ui.content_region_avail();
    ui.child_window("##clip_list")
        .size([remaining[0], remaining[1] - 4.0])
        .build(ui, || {
            for (id, name) in &clip_names {
                if !filter_lower.is_empty() && !name.to_lowercase().contains(&filter_lower) {
                    continue;
                }

                let Some(clip) = clip_library.get(*id) else {
                    continue;
                };
                let row = ClipRow {
                    id: *id,
                    clip,
                    is_selected: timeline_state.current_clip_id == Some(*id),
                    reference_count: count_clip_references(reference_counts, *id),
                };
                build_clip_row(ui, world, &row);
            }
        });
}

struct ClipRow<'a> {
    id: SourceClipId,
    clip: &'a EditableAnimationClip,
    is_selected: bool,
    reference_count: usize,
}

fn build_clip_row(ui: &imgui::Ui, world: &World, row: &ClipRow) {
    if ui
        .selectable_config(format_clip_row_label(row))
        .selected(row.is_selected)
        .build()
    {
        world.send_command(TimelineEvent::SelectClip(row.id));
    }
    if ui.is_item_hovered() {
        show_unbound_tracks_tooltip(ui, row.clip);
    }

    build_clip_drag_source(ui, row.id, &row.clip.name);
    build_clip_row_menu(ui, world, row.id, row.reference_count);
}

fn format_clip_row_label(row: &ClipRow) -> String {
    let clip = row.clip;
    let mut label = format!(
        "{} ({:.1}s) [{}]",
        clip.name, clip.duration, row.reference_count
    );

    let source_filename = extract_source_filename(clip);
    if !source_filename.is_empty() {
        label.push_str(&format!(" <{source_filename}>"));
    }
    if !clip.unbound_tracks.is_empty() {
        label.push_str(&format!(" ({} unbound)", clip.unbound_tracks.len()));
    }

    format!("{label}##clip_{}", row.id)
}

fn show_unbound_tracks_tooltip(ui: &imgui::Ui, clip: &EditableAnimationClip) {
    let unbound_track_names = clip.unbound_track_names();
    if unbound_track_names.is_empty() {
        return;
    }

    ui.tooltip_text(format!(
        "Tracks kept for bones this model lacks (not played):\n{}",
        unbound_track_names.join("\n")
    ));
}

fn build_clip_drag_source(ui: &imgui::Ui, clip_id: SourceClipId, _name: &str) {
    let id = clip_id;
    let _source = ui
        .drag_drop_source_config("CLIP_SOURCE")
        .begin_payload(move || id);
}

/// A clip some schedule still plays cannot be deleted, so its Delete entry is disabled.
fn build_clip_row_menu(
    ui: &imgui::Ui,
    world: &World,
    clip_id: SourceClipId,
    reference_count: usize,
) {
    let Some(_row_menu) = ui.begin_popup_context_item() else {
        return;
    };

    if ui.menu_item("Duplicate") {
        world.send_command(ClipBrowserEvent::Duplicate(clip_id));
    }

    let is_unreferenced = reference_count == 0;
    if ui.menu_item_enabled_selected("Delete", None::<&str>, false, is_unreferenced) {
        world.send_command(ClipBrowserEvent::Delete(clip_id));
    }
}

fn extract_source_filename(clip: &EditableAnimationClip) -> &str {
    clip.source_path
        .as_ref()
        .and_then(|path| std::path::Path::new(path).file_name())
        .and_then(|file_name| file_name.to_str())
        .unwrap_or("")
}

fn build_clip_browser_window(
    ui: &imgui::Ui,
    world: &World,
    _: &AssetStorage,
    _: &GraphicsResources,
) {
    let clip_library = world.resource::<ClipLibrary>();
    let mut browser_state = world.resource_mut::<ClipBrowserState>();
    let timeline_state = world.resource::<TimelineState>();
    let layout = world.resource::<LayoutSnapshot>();
    draw_clip_browser_window(
        ui,
        &clip_library,
        &mut browser_state,
        &timeline_state,
        world,
        &layout,
    );
}

crate::ui_window!("clip_browser", Side, 2, build_clip_browser_window);
