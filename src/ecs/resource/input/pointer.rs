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
