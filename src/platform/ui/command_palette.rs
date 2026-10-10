use imgui::{Condition, Key};

use super::key_modifier::{current_key_modifier, KeyModifier};
use crate::asset::AssetStorage;
use crate::ecs::resource::{
    CommandPaletteSession, CommandPaletteState, HierarchyState, UiWidgetState,
};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::phases::event_dispatch::hierarchy::HierarchyEvent;
use crate::ecs::systems::{query_hierarchy_tree, CameraMotion};
use crate::ecs::world::{Entity, World};
use crate::platform::key_bindings::default_bindings;
use crate::platform::ui::shortcuts::format_global_chord;
use crate::platform::ui::theme::colors::{srgb_to_linear, ACCENT, TEXT, TEXT_SECONDARY};
use crate::platform::ui::theme::search_field;
use crate::platform::ui::theme::shadow::draw_window_shadow;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

const PALETTE_WINDOW_TITLE: &str = "Command Palette";
const PALETTE_WIDTH: f32 = 520.0;
const PALETTE_TOP_FRACTION: f32 = 0.12;
const MAX_VISIBLE_CANDIDATES: usize = 12;
const ROW_TEXT_PADDING: f32 = 8.0;
const ROW_ROUNDING: f32 = 6.0;
const SELECTED_ROW_ALPHA: f32 = 0.25;

const MATCH_SCORE: u32 = 1;
const START_BONUS: u32 = 3;
const WORD_START_BONUS: u32 = 2;
const CONSECUTIVE_BONUS: u32 = 1;

pub(crate) struct PaletteKeyBinding {
    pub(crate) key: Key,
    pub(crate) modifier: KeyModifier,
    pub(crate) label: &'static str,
}

pub(crate) const PALETTE_KEY_BINDING: PaletteKeyBinding = PaletteKeyBinding {
    key: Key::K,
    modifier: KeyModifier::Ctrl,
    label: "Command palette",
};

pub enum PaletteItem {
    Command {
        label: &'static str,
        chord: String,
        send: fn(&World),
    },
    Entity {
        name: String,
        entity: Entity,
    },
}

impl PaletteItem {
    fn label(&self) -> &str {
        match self {
            PaletteItem::Command { label, .. } => label,
            PaletteItem::Entity { name, .. } => name,
        }
    }

    fn chord(&self) -> Option<&str> {
        match self {
            PaletteItem::Command { chord, .. } => Some(chord),
            PaletteItem::Entity { .. } => None,
        }
    }
}

enum PaletteOutcome {
    KeepOpen,
    Close,
}

pub fn fuzzy_score(query: &str, candidate: &str) -> Option<u32> {
    let mut query_chars = query.chars().flat_map(char::to_lowercase).peekable();
    let mut score = 0;
    let mut previous_char = None;
    let mut last_match_index = None;

    for (index, candidate_char) in candidate.chars().flat_map(char::to_lowercase).enumerate() {
        let Some(&query_char) = query_chars.peek() else {
            break;
        };
        if candidate_char == query_char {
            score += MATCH_SCORE + match_bonus(index, previous_char, last_match_index);
            last_match_index = Some(index);
            query_chars.next();
        }
        previous_char = Some(candidate_char);
    }

    query_chars.peek().is_none().then_some(score)
}

fn match_bonus(index: usize, previous_char: Option<char>, last_match_index: Option<usize>) -> u32 {
    match previous_char {
        None => START_BONUS,
        Some(previous) if !previous.is_alphanumeric() => WORD_START_BONUS,
        Some(_) if last_match_index.is_some_and(|last| last + 1 == index) => CONSECUTIVE_BONUS,
        Some(_) => 0,
    }
}

pub fn rank_candidates(query: &str, items: &[PaletteItem]) -> Vec<usize> {
    let mut scored: Vec<(usize, u32)> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| fuzzy_score(query, item.label()).map(|score| (index, score)))
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    scored.into_iter().map(|(index, _)| index).collect()
}

fn collect_palette_items(world: &World) -> Vec<PaletteItem> {
    let commands = default_bindings()
        .into_iter()
        .map(|binding| PaletteItem::Command {
            label: binding.label,
            chord: format_global_chord(binding.modifiers, binding.key),
            send: binding.send,
        });

    let hierarchy_state = world.resource::<HierarchyState>();
    let entities = query_hierarchy_tree(world, &hierarchy_state)
        .into_iter()
        .map(|entry| PaletteItem::Entity {
            name: entry.name,
            entity: entry.entity,
        });

    commands.chain(entities).collect()
}

fn execute_palette_item(world: &World, item: &PaletteItem) {
    match item {
        PaletteItem::Command { send, .. } => send(world),
        PaletteItem::Entity { entity, .. } => {
            world.send_command(HierarchyEvent::SelectEntity(*entity));
            world.send_command(CameraEvent::FrameSelection(CameraMotion::Eased));
        }
    }
}

fn toggle_command_palette_on_hotkey(ui: &imgui::Ui, world: &World) {
    let mut state = world.resource_mut::<UiWidgetState>();
    let palette_is_open = matches!(state.command_palette, CommandPaletteState::Open(_));
    let hotkey_available = !ui.io().want_text_input || palette_is_open;
    let hotkey_pressed = current_key_modifier(ui) == PALETTE_KEY_BINDING.modifier
        && ui.is_key_pressed(PALETTE_KEY_BINDING.key);
    if !hotkey_available || !hotkey_pressed {
        return;
    }

    state.command_palette = match state.command_palette {
        CommandPaletteState::Closed => CommandPaletteState::Open(CommandPaletteSession::default()),
        CommandPaletteState::Open(_) => CommandPaletteState::Closed,
    };
}

fn draw_candidate_rows(
    ui: &imgui::Ui,
    items: &[PaletteItem],
    visible: &[usize],
    selection: usize,
) -> Option<usize> {
    let draw_list = ui.get_window_draw_list();
    let row_height = ui.frame_height();
    let text_offset_y = (row_height - ui.text_line_height()) * 0.5;
    let mut clicked_item = None;

    for (row, &item_index) in visible.iter().enumerate() {
        let item = &items[item_index];
        let row_width = ui.content_region_avail()[0];
        if ui.invisible_button(format!("palette_row_{}", row), [row_width, row_height]) {
            clicked_item = Some(item_index);
        }
        let rect_min = ui.item_rect_min();
        let rect_max = ui.item_rect_max();

        if row == selection {
            let accent_fill = [ACCENT[0], ACCENT[1], ACCENT[2], SELECTED_ROW_ALPHA];
            draw_list
                .add_rect(rect_min, rect_max, srgb_to_linear(accent_fill))
                .rounding(ROW_ROUNDING)
                .filled(true)
                .build();
        }

        let text_y = rect_min[1] + text_offset_y;
        draw_list.add_text(
            [rect_min[0] + ROW_TEXT_PADDING, text_y],
            srgb_to_linear(TEXT),
            item.label(),
        );
        if let Some(chord) = item.chord() {
            let chord_width = ui.calc_text_size(chord)[0];
            draw_list.add_text(
                [rect_max[0] - ROW_TEXT_PADDING - chord_width, text_y],
                srgb_to_linear(TEXT_SECONDARY),
                chord,
            );
        }
    }

    clicked_item
}

fn handle_palette_navigation(
    ui: &imgui::Ui,
    world: &World,
    session: &mut CommandPaletteSession,
    items: &[PaletteItem],
    visible: &[usize],
) -> PaletteOutcome {
    if ui.is_key_pressed(Key::DownArrow) {
        session.selection = (session.selection + 1).min(visible.len().saturating_sub(1));
    }
    if ui.is_key_pressed(Key::UpArrow) {
        session.selection = session.selection.saturating_sub(1);
    }

    let enter_pressed = ui.is_key_pressed(Key::Enter) || ui.is_key_pressed(Key::KeypadEnter);
    match visible.get(session.selection) {
        Some(&item_index) if enter_pressed => {
            execute_palette_item(world, &items[item_index]);
            PaletteOutcome::Close
        }
        _ => PaletteOutcome::KeepOpen,
    }
}

fn draw_palette_contents(
    ui: &imgui::Ui,
    world: &World,
    session: &mut CommandPaletteSession,
) -> PaletteOutcome {
    if ui.is_key_pressed(Key::Escape) {
        if session.input.is_empty() {
            return PaletteOutcome::Close;
        }
        session.input.clear();
        session.selection = 0;
    }

    if !ui.is_any_item_active() {
        ui.set_keyboard_focus_here();
    }
    if search_field(
        ui,
        "command_palette_input",
        "Run a command or select an entity",
        &mut session.input,
    ) {
        session.selection = 0;
    }

    let items = collect_palette_items(world);
    let visible: Vec<usize> = rank_candidates(&session.input, &items)
        .into_iter()
        .take(MAX_VISIBLE_CANDIDATES)
        .collect();
    session.selection = session.selection.min(visible.len().saturating_sub(1));

    if let Some(item_index) = draw_candidate_rows(ui, &items, &visible, session.selection) {
        execute_palette_item(world, &items[item_index]);
        return PaletteOutcome::Close;
    }

    handle_palette_navigation(ui, world, session, &items, &visible)
}

fn build_command_palette_window(
    ui: &imgui::Ui,
    world: &World,
    _assets: &AssetStorage,
    _graphics: &GraphicsResources,
) {
    toggle_command_palette_on_hotkey(ui, world);
    let CommandPaletteState::Open(mut session) =
        world.resource::<UiWidgetState>().command_palette.clone()
    else {
        return;
    };

    let display_size = ui.io().display_size;
    let outcome = ui
        .window(PALETTE_WINDOW_TITLE)
        .position(
            [
                display_size[0] * 0.5,
                display_size[1] * PALETTE_TOP_FRACTION,
            ],
            Condition::Always,
        )
        .position_pivot([0.5, 0.0])
        .size([PALETTE_WIDTH, 0.0], Condition::Always)
        .no_decoration()
        .save_settings(false)
        .build(|| {
            draw_window_shadow(ui, ui.clone_style().window_rounding);
            draw_palette_contents(ui, world, &mut session)
        })
        .unwrap_or(PaletteOutcome::KeepOpen);

    world.resource_mut::<UiWidgetState>().command_palette = match outcome {
        PaletteOutcome::KeepOpen => CommandPaletteState::Open(session),
        PaletteOutcome::Close => CommandPaletteState::Closed,
    };
}

crate::ui_window!("command_palette", Floating, 3, build_command_palette_window);

#[cfg(test)]
mod tests {
    use super::*;

    fn command(label: &'static str) -> PaletteItem {
        PaletteItem::Command {
            label,
            chord: String::new(),
            send: |_| {},
        }
    }

    #[test]
    fn test_empty_query_scores_zero() {
        assert_eq!(fuzzy_score("", "Save scene"), Some(0));
    }

    #[test]
    fn test_subsequence_matches_ignoring_case() {
        assert!(fuzzy_score("svs", "Save scene").is_some());
        assert!(fuzzy_score("SAVE", "save scene").is_some());
    }

    #[test]
    fn test_missing_character_does_not_match() {
        assert_eq!(fuzzy_score("xyz", "Save scene"), None);
        assert_eq!(fuzzy_score("sss", "Save scene"), None);
    }

    #[test]
    fn test_prefix_match_ranks_above_middle_match() {
        let items = [command("Reset scene"), command("Scene settings")];
        assert_eq!(rank_candidates("sce", &items), vec![1, 0]);
    }

    #[test]
    fn test_word_start_scores_above_inner_match() {
        let word_start = fuzzy_score("s", "Save scene").unwrap();
        let inner = fuzzy_score("s", "Undo last").unwrap();
        assert!(word_start > inner);
        assert!(fuzzy_score("f", "Frame selection") > fuzzy_score("f", "Self frame"));
    }

    #[test]
    fn test_consecutive_match_scores_above_scattered_match() {
        let consecutive = fuzzy_score("sav", "Save scene").unwrap();
        let scattered = fuzzy_score("sve", "Save scene").unwrap();
        assert!(consecutive > scattered);
    }

    #[test]
    fn test_rank_excludes_non_matching_candidates() {
        let items = [command("Undo"), command("Redo"), command("Save scene")];
        assert_eq!(rank_candidates("undo", &items), vec![0]);
    }

    #[test]
    fn test_equal_scores_keep_original_order() {
        let items = [command("Apple"), command("Apricot"), command("Avocado")];
        assert_eq!(rank_candidates("a", &items), vec![0, 1, 2]);
        assert_eq!(rank_candidates("", &items), vec![0, 1, 2]);
    }
}
