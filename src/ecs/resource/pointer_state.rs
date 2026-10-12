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

/// Farther than this between press and release, a middle button gesture is a pan, not a click.
pub const CLICK_MAX_TRAVEL_PIXELS: f32 = 4.0;

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
    pub middle_press_position: Option<[f32; 2]>,
}

impl PointerState {
    pub fn middle_clicked_without_drag(&self) -> bool {
        let Some(pressed_at) = self.middle_press_position else {
            return false;
        };
        let travel = ((self.position[0] - pressed_at[0]).powi(2)
            + (self.position[1] - pressed_at[1]).powi(2))
        .sqrt();
        self.middle.just_released() && travel <= CLICK_MAX_TRAVEL_PIXELS
    }
}
