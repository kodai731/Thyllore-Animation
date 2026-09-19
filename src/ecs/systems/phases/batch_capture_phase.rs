use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::BatchRun;
use crate::ecs::systems::world::{batch_run_record_capture, capture_output_path};
use crate::hooks::batch_capture::{BatchCaptureHooks, CaptureContext};

/// Post-render: with the GPU idle, runs every registered readback, then takes the schedule's
/// screenshot and records it.
pub unsafe fn run_batch_capture_phase(
    ctx: &CaptureContext,
    hooks: &BatchCaptureHooks,
) -> Result<()> {
    let output_path = {
        let batch = ctx.world.resource::<BatchRun>();
        capture_output_path(&batch.capture, ctx.slot.index)
    };

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    ctx.device.device.device_wait_idle()?;
    hooks.run_all(ctx);

    let saved = ctx
        .save_screenshot_to(&output_path)
        .map_err(|error| format!("{error:?}"));
    batch_run_record_capture(ctx.world, saved);
    Ok(())
}
