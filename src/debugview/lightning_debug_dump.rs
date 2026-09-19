use anyhow::Result;

use crate::ecs::resource::LightningDebugCapture;
use crate::ecs::systems::{
    build_lightning_debug_record, current_unix_time, lightning_debug_screenshot_path,
    write_lightning_debug_dump,
};
use crate::hooks::batch_capture::{BatchCapture, CaptureContext};

impl BatchCapture for LightningDebugCapture {
    unsafe fn capture(&self, ctx: &CaptureContext) -> Result<()> {
        dump_lightning_debug(ctx);
        Ok(())
    }
}

crate::batch_capture!(LightningDebugCapture);
crate::capture_action!("dump_lightning_debug", LightningDebugCapture);

pub fn dump_lightning_debug(ctx: &CaptureContext) {
    let unix_time = current_unix_time();

    let screenshot_path = lightning_debug_screenshot_path(unix_time);
    let saved_screenshot = match unsafe { ctx.save_screenshot_to(&screenshot_path) } {
        Ok(_) => Some(screenshot_path.as_path()),
        Err(error) => {
            log_warn!("lightning debug screenshot failed: {:?}", error);
            None
        }
    };

    let record = build_lightning_debug_record(ctx.world, saved_screenshot, unix_time);
    match write_lightning_debug_dump(&record, unix_time) {
        Ok(path) => msg_info!("Lightning debug dumped: {}", path.display()),
        Err(error) => log_error!("lightning debug dump failed: {}", error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::resource::{BatchRun, CaptureSchedule};
    use crate::ecs::systems::{batch_apply_debug_actions, resolve_engine_cli_overrides};
    use crate::ecs::world::World;

    #[test]
    fn dump_action_requests_the_capture_inside_a_batch_run() {
        let args: Vec<String> = ["bin", "--batch-debug-action", "dump_lightning_debug"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let actions = resolve_engine_cli_overrides(&args).unwrap().debug_actions;
        assert_eq!(actions[0].name(), "dump_lightning_debug");

        let mut world = World::new();
        world.insert_resource(BatchRun::new(CaptureSchedule::single("out".into(), 0)));
        batch_apply_debug_actions(&mut world, &[actions[0].as_ref()]);
        assert!(world.contains_resource::<LightningDebugCapture>());
    }
}
