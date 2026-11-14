#[derive(Default)]
pub struct PointerCapture {
    pub active: bool,
}

crate::startup_resource!(PointerCapture, Editor);
