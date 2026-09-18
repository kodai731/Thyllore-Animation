use anyhow::Result;

use crate::app::App;
use crate::ecs::resource::WindBatchCapture;
use crate::ecs::systems::{
    build_wind_debug_record, current_unix_time, wind_debug_screenshot_path, write_wind_debug_dump,
};
use crate::hooks::batch_capture::CaptureContext;

unsafe fn wind_debug_capture(ctx: &CaptureContext) -> Result<()> {
    let requested = ctx
        .world
        .get_resource::<WindBatchCapture>()
        .is_some_and(|request| request.debug_dump);
    if requested {
        dump_wind_debug(ctx);
    }
    Ok(())
}

crate::batch_capture!("wind_debug", wind_debug_capture);

pub fn dump_wind_debug(ctx: &CaptureContext) {
    let unix_time = current_unix_time();

    let screenshot_path = wind_debug_screenshot_path(unix_time);
    let saved_screenshot = match unsafe { ctx.save_screenshot_to(&screenshot_path) } {
        Ok(_) => Some(screenshot_path.as_path()),
        Err(error) => {
            log_warn!("wind debug screenshot failed: {:?}", error);
            None
        }
    };

    let record = build_wind_debug_record(ctx.world, saved_screenshot, unix_time);
    match write_wind_debug_dump(&record, unix_time) {
        Ok(path) => msg_info!("Wind debug dumped: {}", path.display()),
        Err(error) => log_error!("wind debug dump failed: {}", error),
    }
}

impl App {
    pub fn dump_wind_debug(&self) {
        dump_wind_debug(&self.interactive_capture_context());
    }
}
