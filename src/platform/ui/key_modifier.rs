use crate::platform::key_bindings::ModifierKeys;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyModifier {
    None,
    Shift,
    Ctrl,
}

pub(crate) fn current_key_modifier(ui: &imgui::Ui) -> KeyModifier {
    let io = ui.io();
    if io.key_ctrl() {
        KeyModifier::Ctrl
    } else if io.key_shift() {
        KeyModifier::Shift
    } else {
        KeyModifier::None
    }
}

impl From<KeyModifier> for ModifierKeys {
    fn from(modifier: KeyModifier) -> Self {
        match modifier {
            KeyModifier::None => ModifierKeys::none(),
            KeyModifier::Shift => ModifierKeys::shift(),
            KeyModifier::Ctrl => ModifierKeys::ctrl(),
        }
    }
}
