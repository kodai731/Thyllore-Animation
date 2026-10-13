use crate::platform::key_bindings::ModifierKeys;

pub(crate) fn current_key_modifier(ui: &imgui::Ui) -> ModifierKeys {
    let io = ui.io();
    ModifierKeys {
        ctrl: io.key_ctrl(),
        shift: io.key_shift(),
        alt: io.key_alt(),
    }
}
