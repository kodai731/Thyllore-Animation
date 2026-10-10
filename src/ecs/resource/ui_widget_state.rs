use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanelVisibility {
    #[default]
    Hidden,
    Shown,
}

#[derive(Clone, Debug, Default)]
pub struct UiWidgetState {
    pub section_open: HashMap<u32, bool>,
    pub scene_panel: PanelVisibility,
}
