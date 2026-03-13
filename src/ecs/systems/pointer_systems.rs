use crate::ecs::resource::{ButtonPhase, ButtonState, RawButtonInput};

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
