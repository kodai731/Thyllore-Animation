use winit::keyboard::Key;

use crate::ecs::systems::phases::event_dispatch::edit_history::EditHistoryEvent;
use crate::ecs::systems::phases::event_dispatch::scene::SceneEvent;
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::world::World;

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
    pub send: fn(&World),
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
            key: "z",
            modifiers: ModifierKeys::ctrl(),
            send: |world| world.send_command(EditHistoryEvent::Undo),
        },
        KeyBinding {
            key: "y",
            modifiers: ModifierKeys::ctrl(),
            send: |world| world.send_command(EditHistoryEvent::Redo),
        },
        KeyBinding {
            key: "s",
            modifiers: ModifierKeys::ctrl(),
            send: |world| world.send_command(SceneEvent::SaveScene),
        },
        KeyBinding {
            key: "s",
            modifiers: ModifierKeys::none(),
            send: |world| world.send_command(TimelineEvent::BoneSetKey),
        },
    ]
}

pub fn dispatch_keyboard_shortcut(
    key: &Key,
    modifiers: ModifierKeys,
    imgui_wants_keyboard: bool,
    bindings: &[KeyBinding],
) -> Option<fn(&World)> {
    for binding in bindings {
        if !binding.matches(key, modifiers) {
            continue;
        }

        if imgui_wants_keyboard && !binding.modifiers.has_any() {
            continue;
        }

        return Some(binding.send);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;

    fn sent_command_names(
        key: &str,
        modifiers: ModifierKeys,
        imgui_wants_keyboard: bool,
    ) -> Vec<String> {
        let bindings = default_bindings();
        let world = world_with_queues();
        if let Some(send) = dispatch_keyboard_shortcut(
            &Key::Character(key.into()),
            modifiers,
            imgui_wants_keyboard,
            &bindings,
        ) {
            send(&world);
        }

        let names = world
            .resource_mut::<UiCommandQueue>()
            .drain()
            .map(|command| format!("{:?}", command))
            .collect();
        names
    }

    fn world_with_queues() -> World {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        world
    }

    #[test]
    fn test_ctrl_s_matches_save_scene() {
        assert_eq!(
            sent_command_names("s", ModifierKeys::ctrl(), false),
            vec!["SaveScene"]
        );
    }

    #[test]
    fn test_plain_s_matches_bone_set_key() {
        assert_eq!(
            sent_command_names("s", ModifierKeys::none(), false),
            vec!["BoneSetKey"]
        );
    }

    #[test]
    fn test_plain_s_blocked_when_imgui_wants_keyboard() {
        assert!(sent_command_names("s", ModifierKeys::none(), true).is_empty());
    }

    #[test]
    fn test_ctrl_s_fires_even_when_imgui_wants_keyboard() {
        assert_eq!(
            sent_command_names("s", ModifierKeys::ctrl(), true),
            vec!["SaveScene"]
        );
    }

    #[test]
    fn test_unbound_key_returns_none() {
        assert!(sent_command_names("z", ModifierKeys::none(), false).is_empty());
    }

    #[test]
    fn test_ctrl_z_matches_undo() {
        assert_eq!(
            sent_command_names("z", ModifierKeys::ctrl(), false),
            vec!["Undo"]
        );
    }
}
