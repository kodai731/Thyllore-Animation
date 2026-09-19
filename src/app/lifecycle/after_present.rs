use anyhow::Result;

use crate::app::App;
use crate::ecs::systems::phases::run_last_phase;

impl App {
    /// Runs once per frame after present. The frame is complete, so this is where readbacks of
    /// the finished image happen: the batch capture, when the schedule asked for one this frame.
    pub unsafe fn after_present(&self, image_index: usize) -> Result<()> {
        run_last_phase(&self.data.ecs_world, |slot| {
            self.capture_context(image_index, slot)
        })
    }
}
