use crate::ecs::resource::FlameUIState;

use super::super::pointer::is_last_item_double_clicked;

/// Canonicalized directory, falling back to the typed text when the path
/// cannot be resolved (broken symlink, permissions, not yet existing).
pub(super) fn canonical_dir_or(dir: &str) -> String {
    std::fs::canonicalize(dir)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| dir.to_string())
}

/// Lightweight validation for the fit path indicator: existence plus a PNG
/// header read (no pixel decode). "ok: ..." prefixed on success.
pub(super) fn validate_texture_fit_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    if !std::path::Path::new(path).is_file() {
        return String::from("file not found");
    }
    match std::fs::File::open(path)
        .map_err(|e| e.to_string())
        .and_then(|file| {
            png::Decoder::new(file)
                .read_info()
                .map_err(|e| e.to_string())
        }) {
        Ok(reader) => {
            let info = reader.info();
            format!("ok: {}x{} {:?}", info.width, info.height, info.color_type)
        }
        Err(error) => format!("not a readable png: {error}"),
    }
}

const TEXTURE_FIT_BROWSER_MAX_ENTRIES: usize = 2000;

/// In-app file browser for the fit texture (G9): breadcrumb + direct path
/// input, png filter, directory-first listing, double-click to descend /
/// confirm. Selection only fills the path field — applying stays on the
/// explicit Apply button. Unreadable entries render disabled instead of
/// failing the listing.
pub(super) fn build_texture_fit_browser(ui: &imgui::Ui, flame_ui: &mut FlameUIState) {
    if !flame_ui.texture_fit_browser_open {
        return;
    }
    let mut open = true;
    let mut confirmed: Option<String> = None;
    ui.window("Select Fit Texture")
        .size([560.0, 430.0], imgui::Condition::FirstUseEver)
        .opened(&mut open)
        .build(|| {
            let dir_now = flame_ui.texture_fit_browser_dir.clone();
            let mut jump: Option<String> = None;

            if ui.small_button("/") {
                jump = Some(String::from("/"));
            }
            let mut accumulated = String::new();
            for (index, part) in dir_now.split('/').filter(|p| !p.is_empty()).enumerate() {
                accumulated.push('/');
                accumulated.push_str(part);
                ui.same_line();
                if ui.small_button(format!("{part}##crumb{index}")) {
                    jump = Some(accumulated.clone());
                }
            }

            ui.input_text("##fit_browser_dir", &mut flame_ui.texture_fit_browser_dir)
                .build();
            ui.same_line();
            if ui.small_button("Go") {
                jump = Some(flame_ui.texture_fit_browser_dir.clone());
            }
            ui.checkbox("all files", &mut flame_ui.texture_fit_browser_show_all);
            ui.same_line();
            ui.checkbox("hidden", &mut flame_ui.texture_fit_browser_show_hidden);
            ui.same_line();
            if ui.small_button("Up") {
                if let Some(parent) = std::path::Path::new(&dir_now).parent() {
                    jump = Some(parent.display().to_string());
                }
            }

            ui.child_window("##fit_browser_list")
                .size([0.0, -34.0])
                .build(ui, || {
                    let read = match std::fs::read_dir(&dir_now) {
                        Ok(read) => read,
                        Err(error) => {
                            ui.text_colored(
                                [0.9, 0.3, 0.3, 1.0],
                                format!("cannot read directory: {error}"),
                            );
                            return;
                        }
                    };
                    let mut rows: Vec<(String, bool, Option<u64>)> = Vec::new();
                    let mut truncated = false;
                    for entry in read.flatten() {
                        let name = match entry.file_name().into_string() {
                            Ok(name) => name,
                            Err(_) => continue,
                        };
                        if !flame_ui.texture_fit_browser_show_hidden && name.starts_with('.') {
                            continue;
                        }
                        let metadata = entry.metadata().ok();
                        let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());
                        if !is_dir
                            && !flame_ui.texture_fit_browser_show_all
                            && !name.to_ascii_lowercase().ends_with(".png")
                        {
                            continue;
                        }
                        if rows.len() >= TEXTURE_FIT_BROWSER_MAX_ENTRIES {
                            truncated = true;
                            break;
                        }
                        rows.push((name, is_dir, metadata.map(|m| m.len())));
                    }
                    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                    for (name, is_dir, size) in &rows {
                        let label = if *is_dir {
                            format!("{name}/")
                        } else if let Some(size) = size {
                            format!("{name}  ({:.1} KB)", *size as f64 / 1024.0)
                        } else {
                            format!("{name}  (unreadable)")
                        };
                        if size.is_none() && !is_dir {
                            ui.text_disabled(label);
                            continue;
                        }
                        let selected = !is_dir && *name == flame_ui.texture_fit_browser_selected;
                        let clicked = ui.selectable_config(&label).selected(selected).build();
                        let double_clicked = is_last_item_double_clicked(ui);
                        if *is_dir {
                            if double_clicked {
                                jump = Some(format!("{}/{}", dir_now.trim_end_matches('/'), name));
                            }
                        } else {
                            if clicked {
                                flame_ui.texture_fit_browser_selected = name.clone();
                            }
                            if double_clicked {
                                confirmed =
                                    Some(format!("{}/{}", dir_now.trim_end_matches('/'), name));
                            }
                        }
                    }
                    if truncated {
                        ui.text_colored(
                            [0.9, 0.7, 0.3, 1.0],
                            format!("listing capped at {TEXTURE_FIT_BROWSER_MAX_ENTRIES} entries"),
                        );
                    }
                });

            let has_selection = !flame_ui.texture_fit_browser_selected.is_empty();
            {
                let _enabled_scope = ui.begin_disabled_with_cond(!has_selection);
                if ui.button("Open") {
                    confirmed = Some(format!(
                        "{}/{}",
                        dir_now.trim_end_matches('/'),
                        flame_ui.texture_fit_browser_selected
                    ));
                }
            }
            ui.same_line();
            if ui.button("Cancel") {
                flame_ui.texture_fit_browser_open = false;
            }

            if let Some(target) = jump {
                flame_ui.texture_fit_browser_dir = canonical_dir_or(&target);
                flame_ui.texture_fit_browser_selected.clear();
            }
        });
    if let Some(path) = confirmed {
        flame_ui.texture_fit_path = path;
        flame_ui.texture_fit_browser_open = false;
    }
    if !open {
        flame_ui.texture_fit_browser_open = false;
    }
}
