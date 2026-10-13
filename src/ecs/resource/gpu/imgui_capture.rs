#[derive(Clone, Debug, Default)]
pub struct ImGuiInputCapture {
    pub wants_mouse: bool,
    pub wants_keyboard: bool,
}

crate::startup_resource!(ImGuiInputCapture, Editor);
