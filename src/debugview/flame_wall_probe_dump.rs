use anyhow::Result;

use crate::ecs::resource::{FlameFieldTraceCapture, FlameWallProbeCapture};
use crate::ecs::systems::{
    batch_run_flame_dump, perform_flame_wall_probe_dump, BATCH_VIEWPORT_SIZE,
};
use crate::hooks::batch_capture::{BatchCapture, CaptureContext};

impl BatchCapture for FlameWallProbeCapture {
    unsafe fn capture(&self, ctx: &CaptureContext) -> Result<()> {
        perform_flame_wall_probe_dump(ctx.world, BATCH_VIEWPORT_SIZE);
        Ok(())
    }
}

impl BatchCapture for FlameFieldTraceCapture {
    unsafe fn capture(&self, ctx: &CaptureContext) -> Result<()> {
        batch_run_flame_dump(
            ctx.world,
            self.trace_path.as_deref(),
            self.wall_probe_path.as_deref(),
        );
        Ok(())
    }
}

crate::batch_capture!(FlameWallProbeCapture);
crate::batch_capture!(FlameFieldTraceCapture);
crate::capture_action!("dump_wall_probe", FlameWallProbeCapture);
