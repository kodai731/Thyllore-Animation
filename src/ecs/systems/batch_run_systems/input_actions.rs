use anyhow::{anyhow, Result};
use imgui::Key;

use crate::ecs::resource::{InjectedInput, InjectedKeyPress, KeyChordModifiers};
use crate::ecs::world::World;

use super::batch_action::BatchAction;

const PRESS_KEY_PREFIX: &str = "press_key=";
const FOCUS_WINDOW_PREFIX: &str = "focus_window=";

#[derive(Debug)]
pub struct PressKey(pub InjectedKeyPress);

#[derive(Debug)]
pub struct FocusWindow(pub String);

impl BatchAction for PressKey {
    fn name(&self) -> &'static str {
        "press_key"
    }
    fn apply(&self, world: &mut World) {
        world
            .resource_mut::<InjectedInput>()
            .pending_presses
            .push(self.0);
    }
}

impl BatchAction for FocusWindow {
    fn name(&self) -> &'static str {
        "focus_window"
    }
    fn apply(&self, world: &mut World) {
        world.resource_mut::<InjectedInput>().pending_window_focus = Some(self.0.clone());
    }
}

/// `Ctrl+Shift+Delete`: zero or more modifiers, then an `imgui::Key` variant name.
pub fn parse_key_chord(text: &str) -> Result<InjectedKeyPress> {
    let mut tokens: Vec<&str> = text.split('+').map(str::trim).collect();
    let key_name = tokens
        .pop()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| anyhow!("key chord '{text}' names no key"))?;
    let key = parse_key_name(key_name)?;

    let mut modifiers = KeyChordModifiers::default();
    for token in tokens {
        match token.to_ascii_lowercase().as_str() {
            "ctrl" => modifiers.ctrl = true,
            "shift" => modifiers.shift = true,
            "alt" => modifiers.alt = true,
            other => return Err(anyhow!("unknown modifier '{other}' in key chord '{text}'")),
        }
    }

    Ok(InjectedKeyPress { key, modifiers })
}

fn parse_key_name(name: &str) -> Result<Key> {
    serde_json::from_value(serde_json::Value::String(name.to_string()))
        .map_err(|_| anyhow!("unknown key '{name}', expected an imgui::Key variant name"))
}

fn press_key_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let chord = text.strip_prefix(PRESS_KEY_PREFIX)?;
    Some(parse_key_chord(chord).map(|press| Box::new(PressKey(press)) as Box<dyn BatchAction>))
}

fn focus_window_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let title = text.strip_prefix(FOCUS_WINDOW_PREFIX)?.trim();
    Some(if title.is_empty() {
        Err(anyhow!("{FOCUS_WINDOW_PREFIX} needs a window title"))
    } else {
        Ok(Box::new(FocusWindow(title.to_string())) as Box<dyn BatchAction>)
    })
}

crate::batch_action!("press_key", press_key_parse);
crate::batch_action!("focus_window", focus_window_parse);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chord_names_its_key_and_modifiers() {
        let press = parse_key_chord("Ctrl+Shift+Delete").expect("valid chord");
        assert_eq!(press.key, Key::Delete);
        assert!(press.modifiers.ctrl);
        assert!(press.modifiers.shift);
        assert!(!press.modifiers.alt);
        assert_eq!(
            press.keys_in_press_order(),
            vec![Key::ModCtrl, Key::ModShift, Key::Delete]
        );
    }

    #[test]
    fn a_bare_key_has_no_modifiers() {
        let press = parse_key_chord("V").expect("valid chord");
        assert_eq!(press.key, Key::V);
        assert_eq!(press.modifiers, KeyChordModifiers::default());
    }

    #[test]
    fn unknown_keys_and_modifiers_are_rejected() {
        assert!(parse_key_chord("Nope").is_err());
        assert!(parse_key_chord("Meta+V").is_err());
        assert!(parse_key_chord("Ctrl+").is_err());
    }

    #[test]
    fn actions_queue_into_the_injected_input_resource() {
        let mut world = World::new();
        world.insert_resource(InjectedInput::default());

        let press = press_key_parse("press_key=Alt+F")
            .expect("prefix")
            .expect("valid");
        press.apply(&mut world);
        let focus = focus_window_parse("focus_window=Hierarchy")
            .expect("prefix")
            .expect("valid");
        focus.apply(&mut world);

        let injected = world.resource::<InjectedInput>();
        assert_eq!(injected.pending_presses.len(), 1);
        assert_eq!(injected.pending_presses[0].key, Key::F);
        assert!(injected.pending_presses[0].modifiers.alt);
        assert_eq!(injected.pending_window_focus.as_deref(), Some("Hierarchy"));
    }

    #[test]
    fn an_empty_window_title_is_rejected() {
        assert!(focus_window_parse("focus_window=")
            .expect("prefix")
            .is_err());
        assert!(press_key_parse("press_key=").expect("prefix").is_err());
    }
}
