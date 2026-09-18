use anyhow::Result;

use crate::app::App;
use crate::ecs::systems::phases::run_batch_capture_phase;
use crate::ecs::systems::requested_capture;
use crate::hooks::batch_capture::{BatchCaptureHooks, CaptureContext, CaptureSlot};
use crate::vulkanr::context::CommandState;

impl App {
    /// Called once per frame after present; does nothing unless the batch schedule asked for
    /// a capture this frame.
    pub unsafe fn run_batch_capture_phase(&self, image_index: usize) -> Result<()> {
        let Some(slot) = requested_capture(&self.data.ecs_world) else {
            return Ok(());
        };
        let hooks = BatchCaptureHooks::collect()?;
        let ctx = self.capture_context(image_index, slot);
        run_batch_capture_phase(&ctx, &hooks)
    }

    pub fn capture_context(&self, image_index: usize, slot: CaptureSlot) -> CaptureContext<'_> {
        CaptureContext {
            instance: &self.instance,
            device: &self.rrdevice,
            command_pool: self.resource::<CommandState>().pool.command_pool,
            world: &self.data.ecs_world,
            graphics: &self.data.graphics_resources,
            raytracing: &self.data.raytracing,
            hdr: self.data.viewport.hdr_buffer.as_ref(),
            image_index,
            slot,
        }
    }

    pub fn interactive_capture_context(&self) -> CaptureContext<'_> {
        self.capture_context(
            self.frame % crate::app::init::MAX_FRAMES_IN_FLIGHT,
            CaptureSlot::interactive(),
        )
    }
}
