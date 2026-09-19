use crate::app::App;
use crate::hooks::batch_capture::{BatchCapture, CaptureContext, CaptureSlot};
use crate::vulkanr::context::CommandState;

impl App {
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

    /// Runs one readback right away, outside the batch schedule (a debug window button).
    pub fn capture_now(&self, capture: &dyn BatchCapture) {
        if let Err(error) = unsafe { capture.capture(&self.interactive_capture_context()) } {
            log_warn!("capture failed: {:?}", error);
        }
    }
}
