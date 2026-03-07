use winit::keyboard::Key;

use crate::ecs::events::UIEvent;

pub struct KeyBinding {
    pub key: &'static str,
    pub ctrl: bool,
    pub shift: bool,
    pub make_event: fn() -> UIEvent,
}

impl KeyBinding {
    fn has_modifier(&self) -> bool {
        self.ctrl || self.shift
    }

    fn matches(&self, key: &Key, ctrl: bool, shift: bool) -> bool {
        if self.ctrl != ctrl || self.shift != shift {
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
            ctrl: true,
            shift: false,
            make_event: || UIEvent::SaveScene,
        },
        KeyBinding {
            key: "s",
            ctrl: false,
            shift: false,
            make_event: || UIEvent::BoneSetKey,
        },
    ]
}

pub fn dispatch_keyboard_shortcut(
    key: &Key,
    ctrl: bool,
    shift: bool,
    imgui_wants_keyboard: bool,
    bindings: &[KeyBinding],
) -> Option<UIEvent> {
    for binding in bindings {
        if !binding.matches(key, ctrl, shift) {
            continue;
        }

        if imgui_wants_keyboard && !binding.has_modifier() {
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

        let result = dispatch_keyboard_shortcut(&key, true, false, false, &bindings);
        assert!(matches!(result, Some(UIEvent::SaveScene)));
    }

    #[test]
    fn test_plain_s_matches_bone_set_key() {
        let bindings = default_bindings();
        let key = Key::Character("s".into());

        let result = dispatch_keyboard_shortcut(&key, false, false, false, &bindings);
        assert!(matches!(result, Some(UIEvent::BoneSetKey)));
    }

    #[test]
    fn test_plain_s_blocked_when_imgui_wants_keyboard() {
        let bindings = default_bindings();
        let key = Key::Character("s".into());

        let result = dispatch_keyboard_shortcut(&key, false, false, true, &bindings);
        assert!(result.is_none());
    }

    #[test]
    fn test_ctrl_s_fires_even_when_imgui_wants_keyboard() {
        let bindings = default_bindings();
        let key = Key::Character("s".into());

        let result = dispatch_keyboard_shortcut(&key, true, false, true, &bindings);
        assert!(matches!(result, Some(UIEvent::SaveScene)));
    }

    #[test]
    fn test_unbound_key_returns_none() {
        let bindings = default_bindings();
        let key = Key::Character("z".into());

        let result = dispatch_keyboard_shortcut(&key, false, false, false, &bindings);
        assert!(result.is_none());
    }
}
