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
    use crate::ecs::resource::{PointerState, CLICK_MAX_TRAVEL_PIXELS};

    fn pointer_after_middle_press_and_release(release_at: [f32; 2]) -> PointerState {
        let mut pointer = PointerState::default();
        button_state_advance(&mut pointer.middle, RawButtonInput::Pressed);
        pointer.middle_press_position = Some([10.0, 10.0]);
        button_state_advance(&mut pointer.middle, RawButtonInput::Released);
        pointer.position = release_at;
        pointer
    }

    #[test]
    fn a_middle_release_near_the_press_is_a_click() {
        let pointer = pointer_after_middle_press_and_release([11.0, 12.0]);
        assert!(pointer.middle_clicked_without_drag());
    }

    #[test]
    fn a_middle_release_after_a_drag_is_not_a_click() {
        let far = 10.0 + CLICK_MAX_TRAVEL_PIXELS * 2.0;
        let pointer = pointer_after_middle_press_and_release([far, 10.0]);
        assert!(!pointer.middle_clicked_without_drag());
    }

    #[test]
    fn a_held_middle_button_is_not_a_click() {
        let mut pointer = PointerState::default();
        button_state_advance(&mut pointer.middle, RawButtonInput::Pressed);
        pointer.middle_press_position = Some([10.0, 10.0]);
        pointer.position = [10.0, 10.0];
        assert!(!pointer.middle_clicked_without_drag());
    }

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
