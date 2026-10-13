#[derive(Clone, Debug, Default)]
pub struct KeyboardModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

crate::startup_resource!(KeyboardModifiers, Editor);
