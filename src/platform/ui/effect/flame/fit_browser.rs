use crate::asset::AssetStorage;
use crate::ecs::resource::FlameUIState;
use crate::ecs::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use crate::platform::ui::pointer::is_last_item_double_clicked;

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

struct BrowserEntry {
    name: String,
    is_dir: bool,
    size: Option<u64>,
}

impl BrowserEntry {
    fn label(&self) -> String {
        match (self.is_dir, self.size) {
            (true, _) => format!("{}/", self.name),
            (false, Some(size)) => format!("{}  ({:.1} KB)", self.name, size as f64 / 1024.0),
            (false, None) => format!("{}  (unreadable)", self.name),
        }
    }
}

/// Directory-first listing filtered by the browser's hidden / all-files toggles, capped at the entry limit.
fn list_browser_entries(
    dir: &str,
    show_hidden: bool,
    show_all: bool,
) -> std::io::Result<(Vec<BrowserEntry>, bool)> {
    let mut entries = Vec::new();
    let mut truncated = false;
    for entry in std::fs::read_dir(dir)?.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        let metadata = entry.metadata().ok();
        let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());
        if !is_dir && !show_all && !name.to_ascii_lowercase().ends_with(".png") {
            continue;
        }
        if entries.len() >= TEXTURE_FIT_BROWSER_MAX_ENTRIES {
            truncated = true;
            break;
        }
        entries.push(BrowserEntry {
            name,
            is_dir,
            size: metadata.map(|m| m.len()),
        });
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    Ok((entries, truncated))
}

/// What one frame of the browser asked for: a directory to jump to and a file to confirm.
#[derive(Default)]
struct BrowserRequest {
    jump: Option<String>,
    confirmed: Option<String>,
}

fn joined(dir: &str, name: &str) -> String {
    format!("{}/{}", dir.trim_end_matches('/'), name)
}

fn draw_breadcrumbs(ui: &imgui::Ui, dir_now: &str) -> Option<String> {
    let mut jump = None;
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
    jump
}

fn draw_path_controls(
    ui: &imgui::Ui,
    flame_ui: &mut FlameUIState,
    dir_now: &str,
) -> Option<String> {
    let mut jump = None;
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
        if let Some(parent) = std::path::Path::new(dir_now).parent() {
            jump = Some(parent.display().to_string());
        }
    }
    jump
}

fn draw_entry(
    ui: &imgui::Ui,
    flame_ui: &mut FlameUIState,
    dir_now: &str,
    entry: &BrowserEntry,
    request: &mut BrowserRequest,
) {
    let label = entry.label();
    if entry.size.is_none() && !entry.is_dir {
        ui.text_disabled(label);
        return;
    }
    let selected = !entry.is_dir && entry.name == flame_ui.texture_fit_browser_selected;
    let clicked = ui.selectable_config(&label).selected(selected).build();
    let double_clicked = is_last_item_double_clicked(ui);
    if entry.is_dir {
        if double_clicked {
            request.jump = Some(joined(dir_now, &entry.name));
        }
        return;
    }
    if clicked {
        flame_ui.texture_fit_browser_selected = entry.name.clone();
    }
    if double_clicked {
        request.confirmed = Some(joined(dir_now, &entry.name));
    }
}

fn draw_entry_list(
    ui: &imgui::Ui,
    flame_ui: &mut FlameUIState,
    dir_now: &str,
    request: &mut BrowserRequest,
) {
    let listing = list_browser_entries(
        dir_now,
        flame_ui.texture_fit_browser_show_hidden,
        flame_ui.texture_fit_browser_show_all,
    );
    let (entries, truncated) = match listing {
        Ok(listing) => listing,
        Err(error) => {
            ui.text_colored(
                [0.9, 0.3, 0.3, 1.0],
                format!("cannot read directory: {error}"),
            );
            return;
        }
    };
    for entry in &entries {
        draw_entry(ui, flame_ui, dir_now, entry, request);
    }
    if truncated {
        ui.text_colored(
            [0.9, 0.7, 0.3, 1.0],
            format!("listing capped at {TEXTURE_FIT_BROWSER_MAX_ENTRIES} entries"),
        );
    }
}

fn draw_confirm_buttons(
    ui: &imgui::Ui,
    flame_ui: &mut FlameUIState,
    dir_now: &str,
) -> Option<String> {
    let mut confirmed = None;
    let has_selection = !flame_ui.texture_fit_browser_selected.is_empty();
    {
        let _enabled_scope = ui.begin_disabled_with_cond(!has_selection);
        if ui.button("Open") {
            confirmed = Some(joined(dir_now, &flame_ui.texture_fit_browser_selected));
        }
    }
    ui.same_line();
    if ui.button("Cancel") {
        flame_ui.texture_fit_browser_open = false;
    }
    confirmed
}

fn draw_browser_contents(ui: &imgui::Ui, flame_ui: &mut FlameUIState) -> BrowserRequest {
    let dir_now = flame_ui.texture_fit_browser_dir.clone();
    let mut request = BrowserRequest::default();

    request.jump = draw_breadcrumbs(ui, &dir_now);
    if let Some(jump) = draw_path_controls(ui, flame_ui, &dir_now) {
        request.jump = Some(jump);
    }
    ui.child_window("##fit_browser_list")
        .size([0.0, -34.0])
        .build(ui, || draw_entry_list(ui, flame_ui, &dir_now, &mut request));
    if let Some(confirmed) = draw_confirm_buttons(ui, flame_ui, &dir_now) {
        request.confirmed = Some(confirmed);
    }

    if let Some(target) = &request.jump {
        flame_ui.texture_fit_browser_dir = canonical_dir_or(target);
        flame_ui.texture_fit_browser_selected.clear();
    }
    request
}

/// In-app file browser for the fit texture: breadcrumbs, a path field, a png filter and a
/// directory-first listing; a double click descends or confirms, selection only fills the path field.
fn build_texture_fit_browser(ui: &imgui::Ui, flame_ui: &mut FlameUIState) {
    if !flame_ui.texture_fit_browser_open {
        return;
    }
    let mut open = true;
    let mut request = BrowserRequest::default();
    ui.window("Select Fit Texture")
        .size([560.0, 430.0], imgui::Condition::FirstUseEver)
        .opened(&mut open)
        .build(|| request = draw_browser_contents(ui, flame_ui));

    if let Some(path) = request.confirmed {
        flame_ui.texture_fit_path = path;
        flame_ui.texture_fit_browser_open = false;
    }
    if !open {
        flame_ui.texture_fit_browser_open = false;
    }
}

fn build_fit_browser_window(
    ui: &imgui::Ui,
    world: &World,
    _: &AssetStorage,
    _: &GraphicsResources,
) {
    let Some(mut flame_ui) = world.get_resource_mut::<FlameUIState>() else {
        return;
    };
    build_texture_fit_browser(ui, &mut flame_ui);
}

crate::ui_window!("flame_fit_browser", Floating, 8, build_fit_browser_window);
