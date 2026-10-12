use imgui::Key;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyChordModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

/// One key chord a headless run presses as if it came from the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InjectedKeyPress {
    pub key: Key,
    pub modifiers: KeyChordModifiers,
}

impl InjectedKeyPress {
    /// Modifiers first, then the key, in the order the imgui input queue expects them to go down.
    pub fn keys_in_press_order(&self) -> Vec<Key> {
        let mut keys = Vec::with_capacity(4);
        if self.modifiers.ctrl {
            keys.push(Key::ModCtrl);
        }
        if self.modifiers.shift {
            keys.push(Key::ModShift);
        }
        if self.modifiers.alt {
            keys.push(Key::ModAlt);
        }
        keys.push(self.key);
        keys
    }
}

/// Keyboard and focus input queued by batch actions; the platform layer feeds it to imgui.
#[derive(Clone, Debug, Default)]
pub struct InjectedInput {
    pub pending_presses: Vec<InjectedKeyPress>,
    pub held_presses: Vec<InjectedKeyPress>,
    pub pending_window_focus: Option<String>,
}
