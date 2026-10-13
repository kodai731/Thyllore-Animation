/// The imgui window that owned keyboard focus when the last frame's UI was built.
#[derive(Clone, Debug, Default)]
pub struct UiFocusState {
    pub focused_window: Option<String>,
}
