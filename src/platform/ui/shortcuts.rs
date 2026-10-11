use imgui::Condition;

use crate::asset::AssetStorage;
use crate::ecs::resource::{PanelVisibility, UiWidgetState};
use crate::ecs::world::World;
use crate::platform::key_bindings::{default_bindings, ModifierKeys};
use crate::platform::ui::command_palette::PALETTE_KEY_BINDING;
use crate::platform::ui::curve_editor::CURVE_EDITOR_KEY_BINDINGS;
use crate::platform::ui::hierarchy_window::TREE_KEY_BINDINGS;
use crate::platform::ui::key_modifier::KeyModifier;
use crate::platform::ui::scene_overlay::GIZMO_KEY_BINDINGS;
use crate::platform::ui::theme::{property_label, section_header, SectionDefault};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

const SHORTCUTS_WINDOW_TITLE: &str = "Keyboard Shortcuts";
const SHORTCUT_SCOPES: [ShortcutScope; 4] = [
    ShortcutScope::Global,
    ShortcutScope::Hierarchy,
    ShortcutScope::CurveEditor,
    ShortcutScope::SceneGizmo,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutScope {
    Global,
    Hierarchy,
    CurveEditor,
    SceneGizmo,
}

impl ShortcutScope {
    fn label(self) -> &'static str {
        match self {
            ShortcutScope::Global => "Global",
            ShortcutScope::Hierarchy => "Hierarchy",
            ShortcutScope::CurveEditor => "Curve Editor",
            ShortcutScope::SceneGizmo => "Scene Gizmo",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ShortcutEntry {
    pub scope: ShortcutScope,
    pub chord: String,
    pub label: &'static str,
}

pub(crate) fn format_chord(modifier: KeyModifier, key_name: &str) -> String {
    match modifier {
        KeyModifier::None => key_name.to_string(),
        KeyModifier::Shift => format!("Shift+{}", key_name),
        KeyModifier::Ctrl => format!("Ctrl+{}", key_name),
    }
}

pub(crate) fn format_global_chord(modifiers: ModifierKeys, key: &str) -> String {
    let upper = key.to_uppercase();
    match (modifiers.ctrl, modifiers.shift) {
        (false, false) => upper,
        (true, false) => format!("Ctrl+{}", upper),
        (false, true) => format!("Shift+{}", upper),
        (true, true) => format!("Ctrl+Shift+{}", upper),
    }
}

fn imgui_key_name(key: imgui::Key) -> &'static str {
    match key {
        imgui::Key::Space => "Space",
        imgui::Key::Escape => "Esc",
        imgui::Key::Enter => "Enter",
        imgui::Key::Backspace => "Backspace",
        imgui::Key::Delete => "Del",
        imgui::Key::Tab => "Tab",
        imgui::Key::LeftArrow => "Left",
        imgui::Key::RightArrow => "Right",
        imgui::Key::UpArrow => "Up",
        imgui::Key::DownArrow => "Down",
        imgui::Key::PageUp => "PgUp",
        imgui::Key::PageDown => "PgDn",
        imgui::Key::Home => "Home",
        imgui::Key::End => "End",
        imgui::Key::Insert => "Ins",
        imgui::Key::Keypad0 => "0",
        imgui::Key::Keypad1 => "1",
        imgui::Key::Keypad2 => "2",
        imgui::Key::Keypad3 => "3",
        imgui::Key::Keypad4 => "4",
        imgui::Key::Keypad5 => "5",
        imgui::Key::Keypad6 => "6",
        imgui::Key::Keypad7 => "7",
        imgui::Key::Keypad8 => "8",
        imgui::Key::Keypad9 => "9",
        imgui::Key::KeypadDecimal => ".",
        imgui::Key::KeypadDivide => "/",
        imgui::Key::KeypadMultiply => "*",
        imgui::Key::KeypadSubtract => "-",
        imgui::Key::KeypadAdd => "+",
        imgui::Key::KeypadEnter => "Enter",
        imgui::Key::KeypadEqual => "=",
        imgui::Key::A => "A",
        imgui::Key::B => "B",
        imgui::Key::C => "C",
        imgui::Key::D => "D",
        imgui::Key::E => "E",
        imgui::Key::F => "F",
        imgui::Key::G => "G",
        imgui::Key::H => "H",
        imgui::Key::I => "I",
        imgui::Key::J => "J",
        imgui::Key::K => "K",
        imgui::Key::L => "L",
        imgui::Key::M => "M",
        imgui::Key::N => "N",
        imgui::Key::O => "O",
        imgui::Key::P => "P",
        imgui::Key::Q => "Q",
        imgui::Key::R => "R",
        imgui::Key::S => "S",
        imgui::Key::T => "T",
        imgui::Key::U => "U",
        imgui::Key::V => "V",
        imgui::Key::W => "W",
        imgui::Key::X => "X",
        imgui::Key::Y => "Y",
        imgui::Key::Z => "Z",
        imgui::Key::Key1 => "1",
        imgui::Key::Key2 => "2",
        imgui::Key::Key3 => "3",
        imgui::Key::Key4 => "4",
        imgui::Key::Key5 => "5",
        imgui::Key::Key6 => "6",
        imgui::Key::Key7 => "7",
        imgui::Key::Key8 => "8",
        imgui::Key::Key9 => "9",
        imgui::Key::Key0 => "0",
        _ => "?",
    }
}

pub fn collect_shortcuts() -> Vec<ShortcutEntry> {
    let global = default_bindings().into_iter().map(|binding| ShortcutEntry {
        scope: ShortcutScope::Global,
        chord: format_global_chord(binding.modifiers, binding.key),
        label: binding.label,
    });

    let command_palette = std::iter::once(ShortcutEntry {
        scope: ShortcutScope::Global,
        chord: format_chord(
            PALETTE_KEY_BINDING.modifier,
            imgui_key_name(PALETTE_KEY_BINDING.key),
        ),
        label: PALETTE_KEY_BINDING.label,
    });

    let hierarchy = TREE_KEY_BINDINGS.iter().map(|binding| ShortcutEntry {
        scope: ShortcutScope::Hierarchy,
        chord: format_chord(binding.modifier, imgui_key_name(binding.key)),
        label: binding.label,
    });

    let curve_editor = CURVE_EDITOR_KEY_BINDINGS
        .iter()
        .map(|binding| ShortcutEntry {
            scope: ShortcutScope::CurveEditor,
            chord: format_chord(binding.modifier, imgui_key_name(binding.key)),
            label: binding.label,
        });

    let scene_gizmo = GIZMO_KEY_BINDINGS.iter().map(|binding| ShortcutEntry {
        scope: ShortcutScope::SceneGizmo,
        chord: format_chord(KeyModifier::None, imgui_key_name(binding.key)),
        label: binding.label,
    });

    global
        .chain(command_palette)
        .chain(hierarchy)
        .chain(curve_editor)
        .chain(scene_gizmo)
        .collect()
}

fn toggle_shortcuts_panel_on_hotkey(ui: &imgui::Ui, world: &World) {
    let io = ui.io();
    let toggle_pressed = !io.want_text_input()
        && (io.key_shift() || io.key_ctrl())
        && ui.is_key_pressed(imgui::Key::Slash);
    if !toggle_pressed {
        return;
    }

    let mut state = world.resource_mut::<UiWidgetState>();
    state.shortcuts_panel = match state.shortcuts_panel {
        PanelVisibility::Hidden => PanelVisibility::Shown,
        PanelVisibility::Shown => PanelVisibility::Hidden,
    };
}

fn draw_shortcut_sections(ui: &imgui::Ui, world: &World) {
    let shortcuts = collect_shortcuts();
    for scope in SHORTCUT_SCOPES {
        if !section_header(ui, world, scope.label(), SectionDefault::Open) {
            continue;
        }
        for entry in shortcuts.iter().filter(|entry| entry.scope == scope) {
            property_label(ui, &entry.chord);
            ui.text(entry.label);
        }
    }
}

fn build_shortcuts_window(
    ui: &imgui::Ui,
    world: &World,
    _assets: &AssetStorage,
    _graphics: &GraphicsResources,
) {
    toggle_shortcuts_panel_on_hotkey(ui, world);
    if world.resource::<UiWidgetState>().shortcuts_panel == PanelVisibility::Hidden {
        return;
    }

    let display_size = ui.io().display_size();
    let mut is_open = true;
    ui.window(SHORTCUTS_WINDOW_TITLE)
        .position(
            [display_size[0] * 0.5, display_size[1] * 0.5],
            Condition::Always,
        )
        .position_pivot([0.5, 0.5])
        .always_auto_resize(true)
        .collapsible(false)
        .opened(&mut is_open)
        .build(|| draw_shortcut_sections(ui, world));

    if !is_open || ui.is_key_pressed(imgui::Key::Escape) {
        world.resource_mut::<UiWidgetState>().shortcuts_panel = PanelVisibility::Hidden;
    }
}

crate::ui_window!("keyboard_shortcuts", Floating, 2, build_shortcuts_window);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_shortcuts_contains_all_scopes() {
        let shortcuts = collect_shortcuts();
        let scopes: Vec<ShortcutScope> = shortcuts.iter().map(|e| e.scope).collect();
        assert!(scopes.contains(&ShortcutScope::Global));
        assert!(scopes.contains(&ShortcutScope::Hierarchy));
        assert!(scopes.contains(&ShortcutScope::CurveEditor));
        assert!(scopes.contains(&ShortcutScope::SceneGizmo));
    }

    #[test]
    fn test_no_duplicate_chords_within_scope() {
        let shortcuts = collect_shortcuts();

        for scope in SHORTCUT_SCOPES {
            let scope_entries: Vec<&ShortcutEntry> =
                shortcuts.iter().filter(|e| e.scope == scope).collect();
            let mut chords: Vec<String> = scope_entries.iter().map(|e| e.chord.clone()).collect();
            chords.sort();
            chords.dedup();
            assert_eq!(
                chords.len(),
                scope_entries.len(),
                "duplicate chord in {:?} scope",
                scope
            );
        }
    }

    #[test]
    fn test_command_palette_hotkey_is_listed_as_global() {
        let shortcuts = collect_shortcuts();
        assert!(shortcuts
            .iter()
            .any(|entry| entry.scope == ShortcutScope::Global
                && entry.chord == "Ctrl+K"
                && entry.label == PALETTE_KEY_BINDING.label));
    }

    #[test]
    fn test_format_chord_none() {
        assert_eq!(format_chord(KeyModifier::None, "F"), "F");
    }

    #[test]
    fn test_format_chord_shift() {
        assert_eq!(format_chord(KeyModifier::Shift, "E"), "Shift+E");
    }

    #[test]
    fn test_format_chord_ctrl() {
        assert_eq!(format_chord(KeyModifier::Ctrl, "Z"), "Ctrl+Z");
    }
}
