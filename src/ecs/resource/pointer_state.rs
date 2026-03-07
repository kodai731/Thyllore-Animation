#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawButtonInput {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonPhase {
    #[default]
    Released,
    JustPressed,
    Held,
    JustReleased,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ButtonState {
    pub(crate) phase: ButtonPhase,
}

impl ButtonState {
    pub fn just_pressed(&self) -> bool {
        self.phase == ButtonPhase::JustPressed
    }

    pub fn held(&self) -> bool {
        matches!(self.phase, ButtonPhase::JustPressed | ButtonPhase::Held)
    }

    pub fn just_released(&self) -> bool {
        self.phase == ButtonPhase::JustReleased
    }
}

pub fn button_state_advance(state: &mut ButtonState, input: RawButtonInput) {
    state.phase = match (state.phase, input) {
        (ButtonPhase::Released, RawButtonInput::Pressed) => ButtonPhase::JustPressed,
        (ButtonPhase::JustPressed, RawButtonInput::Pressed) => ButtonPhase::Held,
        (ButtonPhase::Held, RawButtonInput::Pressed) => ButtonPhase::Held,
        (ButtonPhase::JustReleased, RawButtonInput::Pressed) => ButtonPhase::JustPressed,
        (ButtonPhase::JustPressed | ButtonPhase::Held, RawButtonInput::Released) => {
            ButtonPhase::JustReleased
        }
        (ButtonPhase::Released | ButtonPhase::JustReleased, RawButtonInput::Released) => {
            ButtonPhase::Released
        }
    };
}

#[derive(Clone, Debug, Default)]
pub struct PointerState {
    pub left: ButtonState,
    pub right: ButtonState,
    pub middle: ButtonState,
    pub position: [f32; 2],
    pub viewport_position: [f32; 2],
    pub wheel_delta: f32,
    pub viewport_hovered: bool,
    pub imgui_wants_pointer: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_state_transitions() {
        let mut btn = ButtonState::default();
        assert!(!btn.just_pressed());
        assert!(!btn.held());

        button_state_advance(&mut btn, RawButtonInput::Pressed);
        assert!(btn.just_pressed());
        assert!(btn.held());

        button_state_advance(&mut btn, RawButtonInput::Pressed);
        assert!(!btn.just_pressed());
        assert!(btn.held());

        button_state_advance(&mut btn, RawButtonInput::Released);
        assert!(btn.just_released());
        assert!(!btn.held());

        button_state_advance(&mut btn, RawButtonInput::Released);
        assert!(!btn.just_released());
        assert!(!btn.held());
    }

    #[test]
    fn test_re_press_from_just_released() {
        let mut btn = ButtonState::default();
        button_state_advance(&mut btn, RawButtonInput::Pressed);
        button_state_advance(&mut btn, RawButtonInput::Released);
        assert!(btn.just_released());

        button_state_advance(&mut btn, RawButtonInput::Pressed);
        assert!(btn.just_pressed());
    }
}
