use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct UiWidgetState {
    pub section_open: HashMap<u32, bool>,
}
