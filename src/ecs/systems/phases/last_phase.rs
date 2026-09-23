use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::BatchRun;
use crate::ecs::systems::world::{
    batch_run_record_capture, capture_output_path, requested_capture,
};
use crate::ecs::World;
use crate::hooks::batch_capture::{BatchCaptureHooks, CaptureContext, CaptureSlot};

/// Post-render: with the GPU idle, runs every registered readback, then takes the schedule's
/// screenshot and records it.
unsafe fn run_batch_capture(ctx: &CaptureContext, hooks: &BatchCaptureHooks) -> Result<()> {
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

pub unsafe fn run_last_phase<'a>(
    world: &'a World,
    build_context: impl FnOnce(CaptureSlot) -> CaptureContext<'a>,
) -> Result<()> {
    let Some(slot) = requested_capture(world) else {
        return Ok(());
    };
    let hooks = BatchCaptureHooks::collect()?;
    let ctx = build_context(slot);
    run_batch_capture(&ctx, &hooks)
}
