use anyhow::{anyhow, Result};
use imgui::Key;

use crate::ecs::resource::{InjectedInput, InjectedKeyPress, KeyChordModifiers};
use crate::ecs::world::World;

use super::batch_action::BatchAction;

const PRESS_KEY_PREFIX: &str = "press_key=";
const FOCUS_WINDOW_PREFIX: &str = "focus_window=";
const MOVE_MOUSE_PREFIX: &str = "move_mouse=";
const CLICK_PREFIX: &str = "click=";

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

#[derive(Debug)]
pub struct MoveMouse(pub [f32; 2]);

#[derive(Debug)]
pub struct Click(pub [f32; 2]);

impl BatchAction for MoveMouse {
    fn name(&self) -> &'static str {
        "move_mouse"
    }
    fn apply(&self, world: &mut World) {
        world.resource_mut::<InjectedInput>().pending_mouse_move = Some(self.0);
    }
}

impl BatchAction for Click {
    fn name(&self) -> &'static str {
        "click"
    }
    fn apply(&self, world: &mut World) {
        world.resource_mut::<InjectedInput>().pending_click = Some(self.0);
    }
}

/// `640,360`: a window-space pixel position.
pub fn parse_mouse_position(text: &str) -> Result<[f32; 2]> {
    let (x, y) = text
        .split_once(',')
        .ok_or_else(|| anyhow!("mouse position '{text}' must be <x>,<y>"))?;
    let parse = |axis: &str| {
        axis.trim()
            .parse::<f32>()
            .map_err(|_| anyhow!("invalid coordinate '{axis}' in mouse position '{text}'"))
    };
    Ok([parse(x)?, parse(y)?])
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

fn move_mouse_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let position = text.strip_prefix(MOVE_MOUSE_PREFIX)?;
    Some(parse_mouse_position(position).map(|p| Box::new(MoveMouse(p)) as Box<dyn BatchAction>))
}

fn click_parse(text: &str) -> Option<Result<Box<dyn BatchAction>>> {
    let position = text.strip_prefix(CLICK_PREFIX)?;
    Some(parse_mouse_position(position).map(|p| Box::new(Click(p)) as Box<dyn BatchAction>))
}

crate::batch_action!("press_key", press_key_parse);
crate::batch_action!("focus_window", focus_window_parse);
crate::batch_action!("move_mouse", move_mouse_parse);
crate::batch_action!("click", click_parse);

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
    fn mouse_actions_queue_their_positions() {
        let mut world = World::new();
        world.insert_resource(InjectedInput::default());

        move_mouse_parse("move_mouse=10,20")
            .expect("prefix")
            .expect("valid")
            .apply(&mut world);
        click_parse("click=30.5, 40")
            .expect("prefix")
            .expect("valid")
            .apply(&mut world);

        let injected = world.resource::<InjectedInput>();
        assert_eq!(injected.pending_mouse_move, Some([10.0, 20.0]));
        assert_eq!(injected.pending_click, Some([30.5, 40.0]));
        assert!(click_parse("click=30").expect("prefix").is_err());
        assert!(click_parse("click=a,b").expect("prefix").is_err());
    }

    #[test]
    fn an_empty_window_title_is_rejected() {
        assert!(focus_window_parse("focus_window=")
            .expect("prefix")
            .is_err());
        assert!(press_key_parse("press_key=").expect("prefix").is_err());
    }
}
