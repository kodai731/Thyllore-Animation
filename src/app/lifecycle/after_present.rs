use anyhow::Result;

use crate::app::App;
use crate::ecs::systems::phases::run_batch_capture_phase;
use crate::ecs::systems::requested_capture;
use crate::hooks::batch_capture::BatchCaptureHooks;

impl App {
    /// Runs once per frame after present. The frame is complete, so this is where readbacks of
    /// the finished image happen: the batch capture, when the schedule asked for one this frame.
    pub unsafe fn after_present(&self, image_index: usize) -> Result<()> {
        let Some(slot) = requested_capture(&self.data.ecs_world) else {
            return Ok(());
        };
        let hooks = BatchCaptureHooks::collect()?;
        let ctx = self.capture_context(image_index, slot);
        run_batch_capture_phase(&ctx, &hooks)
    }
}
