use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanelVisibility {
    #[default]
    Hidden,
    Shown,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandPaletteSession {
    pub input: String,
    pub selection: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CommandPaletteState {
    #[default]
    Closed,
    Open(CommandPaletteSession),
}

#[derive(Clone, Debug, Default)]
pub struct UiWidgetState {
    pub section_open: HashMap<u32, bool>,
    pub scene_panel: PanelVisibility,
    pub shortcuts_panel: PanelVisibility,
    pub command_palette: CommandPaletteState,
}
