/// Set by any system that wants the event loop to stop after the current frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppExit {
    requested: bool,
}

impl AppExit {
    pub fn request(&mut self) {
        self.requested = true;
    }

    pub fn is_requested(&self) -> bool {
        self.requested
    }
}
