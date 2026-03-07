use winit::keyboard::Key;

use crate::ecs::events::UIEvent;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModifierKeys {
    pub ctrl: bool,
    pub shift: bool,
}

impl ModifierKeys {
    pub fn none() -> Self {
        Self {
            ctrl: false,
            shift: false,
        }
    }

    pub fn ctrl() -> Self {
        Self {
            ctrl: true,
            shift: false,
        }
    }

    pub fn has_any(&self) -> bool {
        self.ctrl || self.shift
    }
}

pub struct KeyBinding {
    pub key: &'static str,
    pub modifiers: ModifierKeys,
    pub make_event: fn() -> UIEvent,
}

impl KeyBinding {
    fn matches(&self, key: &Key, modifiers: ModifierKeys) -> bool {
        if self.modifiers != modifiers {
            return false;
        }

        if let Key::Character(ref c) = key {
            c.eq_ignore_ascii_case(self.key)
        } else {
            false
        }
    }
}

pub fn default_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding {
            key: "s",
            modifiers: ModifierKeys::ctrl(),
            make_event: || UIEvent::SaveScene,
        },
        KeyBinding {
            key: "s",
            modifiers: ModifierKeys::none(),
            make_event: || UIEvent::BoneSetKey,
        },
    ]
}

pub fn dispatch_keyboard_shortcut(
    key: &Key,
    modifiers: ModifierKeys,
    imgui_wants_keyboard: bool,
    bindings: &[KeyBinding],
) -> Option<UIEvent> {
    for binding in bindings {
        if !binding.matches(key, modifiers) {
            continue;
        }

        if imgui_wants_keyboard && !binding.modifiers.has_any() {
            continue;
        }

        return Some((binding.make_event)());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctrl_s_matches_save_scene() {
        let bindings = default_bindings();
        let key = Key::Character("s".into());

        let result = dispatch_keyboard_shortcut(&key, ModifierKeys::ctrl(), false, &bindings);
        assert!(matches!(result, Some(UIEvent::SaveScene)));
    }

    #[test]
    fn test_plain_s_matches_bone_set_key() {
        let bindings = default_bindings();
        let key = Key::Character("s".into());

        let result = dispatch_keyboard_shortcut(&key, ModifierKeys::none(), false, &bindings);
        assert!(matches!(result, Some(UIEvent::BoneSetKey)));
    }

    #[test]
    fn test_plain_s_blocked_when_imgui_wants_keyboard() {
        let bindings = default_bindings();
        let key = Key::Character("s".into());

        let result = dispatch_keyboard_shortcut(&key, ModifierKeys::none(), true, &bindings);
        assert!(result.is_none());
    }

    #[test]
    fn test_ctrl_s_fires_even_when_imgui_wants_keyboard() {
        let bindings = default_bindings();
        let key = Key::Character("s".into());

        let result = dispatch_keyboard_shortcut(&key, ModifierKeys::ctrl(), true, &bindings);
        assert!(matches!(result, Some(UIEvent::SaveScene)));
    }

    #[test]
    fn test_unbound_key_returns_none() {
        let bindings = default_bindings();
        let key = Key::Character("z".into());

        let result = dispatch_keyboard_shortcut(&key, ModifierKeys::none(), false, &bindings);
        assert!(result.is_none());
    }
}
