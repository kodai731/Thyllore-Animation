use imgui::Condition;

use crate::animation::editable::SourceClipId;
use crate::asset::AssetStorage;
use crate::ecs::events::{send_dialog_request, ClipExportFormat, DialogRequest};
use crate::ecs::resource::{ClipBrowserState, ClipLibrary, GltfModelCache, TimelineState};
use crate::ecs::systems::phases::event_dispatch::clip_browser::ClipBrowserEvent;
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

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
            build_toolbar(ui, timeline_state, world);
            ui.separator();
            build_filter_bar(ui, browser_state);
            ui.separator();
            build_clip_list(ui, clip_library, browser_state, timeline_state, world);
        });
}

fn build_toolbar(ui: &imgui::Ui, timeline_state: &TimelineState, world: &World) {
    if ui.small_button("+ New") {
        world.send_command(ClipBrowserEvent::CreateEmpty);
    }

    ui.same_line();
    if ui.small_button("Load") {
        send_dialog_request(world, DialogRequest::LoadClip);
    }

    ui.same_line();
    let has_selection = timeline_state.current_clip_id.is_some();
    if has_selection {
        if ui.small_button("Save") {
            if let Some(id) = timeline_state.current_clip_id {
                send_dialog_request(world, DialogRequest::SaveClip(id));
            }
        }
    } else {
        ui.text_disabled("Save");
    }

    ui.same_line();
    if has_selection {
        if ui.small_button("FBX") {
            if let Some(id) = timeline_state.current_clip_id {
                send_dialog_request(
                    world,
                    DialogRequest::ExportClip {
                        source_id: id,
                        format: ClipExportFormat::Fbx,
                    },
                );
            }
        }
    } else {
        ui.text_disabled("FBX");
    }

    ui.same_line();
    let has_model = world
        .get_resource::<GltfModelCache>()
        .map_or(false, |c| c.has_model());
    if has_selection {
        if ui.small_button("glTF") {
            if let Some(id) = timeline_state.current_clip_id {
                send_dialog_request(
                    world,
                    DialogRequest::ExportClip {
                        source_id: id,
                        format: ClipExportFormat::Gltf,
                    },
                );
            }
        }
    } else if has_model {
        if ui.small_button("glTF") {
            send_dialog_request(world, DialogRequest::ExportModelGltf);
        }
    } else {
        ui.text_disabled("glTF");
    }

    ui.same_line();
    if has_selection {
        if ui.small_button("glTF (anim only)") {
            if let Some(id) = timeline_state.current_clip_id {
                send_dialog_request(
                    world,
                    DialogRequest::ExportClip {
                        source_id: id,
                        format: ClipExportFormat::GltfAnimationOnly,
                    },
                );
            }
        }
    } else {
        ui.text_disabled("glTF (anim only)");
    }

    ui.same_line();
    let can_duplicate = has_selection;
    if can_duplicate {
        if ui.small_button("Dup") {
            if let Some(id) = timeline_state.current_clip_id {
                world.send_command(ClipBrowserEvent::Duplicate(id));
            }
        }
    } else {
        ui.text_disabled("Dup");
    }

    ui.same_line();
    if can_duplicate {
        if ui.small_button("Del") {
            if let Some(id) = timeline_state.current_clip_id {
                world.send_command(ClipBrowserEvent::Delete(id));
            }
        }
    } else {
        ui.text_disabled("Del");
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
    world: &World,
) {
    let clip_names =
        crate::ecs::systems::clip_library_systems::clip_library_clip_names(clip_library);

    if clip_names.is_empty() {
        ui.text_disabled("No clips");
        return;
    }

    let reference_counts =
        crate::ecs::systems::clip_library_systems::clip_library_count_references(world);

    let filter_lower = browser_state.filter_text.to_lowercase();

    let remaining = ui.content_region_avail();
    ui.child_window("##clip_list")
        .size([remaining[0], remaining[1] - 4.0])
        .build(|| {
            for (id, name) in &clip_names {
                if !filter_lower.is_empty() && !name.to_lowercase().contains(&filter_lower) {
                    continue;
                }

                let is_selected = timeline_state.current_clip_id == Some(*id);
                let ref_count = reference_counts
                    .iter()
                    .find(|(sid, _)| *sid == *id)
                    .map(|(_, c)| *c)
                    .unwrap_or(0);

                let clip = clip_library.get(*id);
                let duration = clip.map(|c| c.duration).unwrap_or(0.0);
                let source_filename = extract_source_filename(clip);

                let label = if source_filename.is_empty() {
                    format!("{} ({:.1}s) [{}]##clip_{}", name, duration, ref_count, id)
                } else {
                    format!(
                        "{} ({:.1}s) [{}] <{}>##clip_{}",
                        name, duration, ref_count, source_filename, id
                    )
                };

                if ui.selectable_config(&label).selected(is_selected).build() {
                    world.send_command(TimelineEvent::SelectClip(*id));
                }

                build_clip_drag_source(ui, *id, name);
            }
        });
}

fn build_clip_drag_source(ui: &imgui::Ui, clip_id: SourceClipId, _name: &str) {
    let id = clip_id;
    let _source = ui
        .drag_drop_source_config("CLIP_SOURCE")
        .begin_payload(move || id);
}

fn extract_source_filename(
    clip: Option<&crate::animation::editable::EditableAnimationClip>,
) -> String {
    clip.and_then(|c| c.source_path.as_ref())
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
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
