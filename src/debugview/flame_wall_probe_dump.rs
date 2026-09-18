use anyhow::Result;

use crate::ecs::resource::FlameBatchCapture;
use crate::ecs::systems::{
    batch_run_flame_dump, perform_flame_wall_probe_dump, BATCH_VIEWPORT_SIZE,
};
use crate::hooks::batch_capture::CaptureContext;

/// Writes the wall probe and field traces the flame batch flags asked for; World only.
unsafe fn flame_wall_probe_capture(ctx: &CaptureContext) -> Result<()> {
    let Some(request) = ctx
        .world
        .get_resource::<FlameBatchCapture>()
        .map(|request| request.clone())
    else {
        return Ok(());
    };

    if request.wall_probe {
        perform_flame_wall_probe_dump(ctx.world, BATCH_VIEWPORT_SIZE);
    }
    if request.trace_path.is_some() || request.wall_probe_path.is_some() {
        batch_run_flame_dump(
            ctx.world,
            request.trace_path.as_deref(),
            request.wall_probe_path.as_deref(),
        );
    }
    Ok(())
}

crate::batch_capture!("flame_wall_probe", flame_wall_probe_capture);
