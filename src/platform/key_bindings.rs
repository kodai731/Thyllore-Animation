use winit::keyboard::Key;

use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::phases::event_dispatch::edit_history::EditHistoryEvent;
use crate::ecs::systems::phases::event_dispatch::scene::SceneEvent;
use crate::ecs::systems::phases::event_dispatch::scene_camera::SceneCameraEvent;
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::systems::CameraMotion;
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

    pub fn shift() -> Self {
        Self {
            ctrl: false,
            shift: true,
        }
    }

    pub fn has_any(&self) -> bool {
        self.ctrl || self.shift
    }
}

/// Where a binding listens: everywhere, or only while the Scene window holds keyboard focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingScope {
    Global,
    SceneView,
}

/// What the keyboard is currently attached to, sampled once per key event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyboardFocus {
    pub imgui_wants_keyboard: bool,
    pub scene_view_focused: bool,
}

pub struct KeyBinding {
    pub key: &'static str,
    pub modifiers: ModifierKeys,
    pub scope: BindingScope,
    pub label: &'static str,
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
            scope: BindingScope::Global,
            label: "Undo",
            send: |world| world.send_command(EditHistoryEvent::Undo),
        },
        KeyBinding {
            key: "y",
            modifiers: ModifierKeys::ctrl(),
            scope: BindingScope::Global,
            label: "Redo",
            send: |world| world.send_command(EditHistoryEvent::Redo),
        },
        KeyBinding {
            key: "s",
            modifiers: ModifierKeys::ctrl(),
            scope: BindingScope::Global,
            label: "Save scene",
            send: |world| world.send_command(SceneEvent::SaveScene),
        },
        KeyBinding {
            key: "s",
            modifiers: ModifierKeys::none(),
            scope: BindingScope::SceneView,
            label: "Set bone key",
            send: |world| world.send_command(TimelineEvent::BoneSetKey),
        },
        KeyBinding {
            key: "f",
            modifiers: ModifierKeys::none(),
            scope: BindingScope::SceneView,
            label: "Frame selection",
            send: |world| world.send_command(CameraEvent::FrameSelection(CameraMotion::Eased)),
        },
        KeyBinding {
            key: "0",
            modifiers: ModifierKeys::none(),
            scope: BindingScope::SceneView,
            label: "Toggle camera view",
            send: |world| world.send_command(SceneCameraEvent::ToggleCameraView),
        },
    ]
}

pub fn dispatch_keyboard_shortcut(
    key: &Key,
    modifiers: ModifierKeys,
    focus: KeyboardFocus,
    bindings: &[KeyBinding],
) -> Option<fn(&World)> {
    bindings
        .iter()
        .find(|binding| binding.matches(key, modifiers) && binding.listens(focus))
        .map(|binding| binding.send)
}

impl KeyBinding {
    fn listens(&self, focus: KeyboardFocus) -> bool {
        match self.scope {
            BindingScope::Global => true,
            BindingScope::SceneView => focus.scene_view_focused && !focus.imgui_wants_keyboard,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;

    const SCENE_VIEW: KeyboardFocus = KeyboardFocus {
        imgui_wants_keyboard: false,
        scene_view_focused: true,
    };
    const OTHER_WINDOW: KeyboardFocus = KeyboardFocus {
        imgui_wants_keyboard: false,
        scene_view_focused: false,
    };
    const TEXT_INPUT: KeyboardFocus = KeyboardFocus {
        imgui_wants_keyboard: true,
        scene_view_focused: true,
    };

    fn sent_command_names(key: &str, modifiers: ModifierKeys, focus: KeyboardFocus) -> Vec<String> {
        let bindings = default_bindings();
        let world = world_with_queues();
        if let Some(send) =
            dispatch_keyboard_shortcut(&Key::Character(key.into()), modifiers, focus, &bindings)
        {
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
            sent_command_names("s", ModifierKeys::ctrl(), OTHER_WINDOW),
            vec!["SaveScene"]
        );
    }

    #[test]
    fn test_plain_s_matches_bone_set_key_in_the_scene_view() {
        assert_eq!(
            sent_command_names("s", ModifierKeys::none(), SCENE_VIEW),
            vec!["BoneSetKey"]
        );
    }

    #[test]
    fn test_plain_keys_stay_silent_while_another_window_is_focused() {
        assert!(sent_command_names("s", ModifierKeys::none(), OTHER_WINDOW).is_empty());
        assert!(sent_command_names("f", ModifierKeys::none(), OTHER_WINDOW).is_empty());
    }

    #[test]
    fn test_plain_s_blocked_when_imgui_wants_keyboard() {
        assert!(sent_command_names("s", ModifierKeys::none(), TEXT_INPUT).is_empty());
    }

    #[test]
    fn test_ctrl_s_fires_even_when_imgui_wants_keyboard() {
        assert_eq!(
            sent_command_names("s", ModifierKeys::ctrl(), TEXT_INPUT),
            vec!["SaveScene"]
        );
    }

    #[test]
    fn test_unbound_key_returns_none() {
        assert!(sent_command_names("z", ModifierKeys::none(), SCENE_VIEW).is_empty());
    }

    #[test]
    fn test_ctrl_z_matches_undo() {
        assert_eq!(
            sent_command_names("z", ModifierKeys::ctrl(), OTHER_WINDOW),
            vec!["Undo"]
        );
    }
}
