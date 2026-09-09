use crate::app::App;
use crate::ecs::resource::{BatchRun, BatchRunState};
use crate::vulkanr::vulkan::*;

use anyhow::Result;

impl App {
    pub(crate) fn process_batch_screenshot(&mut self, image_index: usize) -> Result<()> {
        let state = self
            .data
            .ecs_world
            .get_resource::<BatchRun>()
            .map(|b| b.state.clone());
        let (dump_wall_probe, dump_water_debug) = self
            .data
            .ecs_world
            .get_resource::<BatchRun>()
            .map(|b| (b.dump_wall_probe, b.dump_water_debug))
            .unwrap_or((false, false));

        if !matches!(state, Some(BatchRunState::ScreenshotRequested)) {
            return Ok(());
        }

        unsafe {
            self.rrdevice.device.device_wait_idle()?;
        }

        if dump_water_debug {
            self.dump_water_debug_at(image_index);
        }

        let save_result = unsafe { self.save_screenshot(image_index) };
        crate::ecs::systems::batch_run_record_screenshot(
            &self.data.ecs_world,
            save_result.map_err(|e| format!("{e:?}")),
        );

        // Wall probe dump: if this batch run was started with
        // --batch-debug-action dump_wall_probe, dump synchronously
        // at the same frame/time as the screenshot.
        if dump_wall_probe {
            crate::ecs::systems::perform_flame_wall_probe_dump(
                &self.data.ecs_world,
                [1680.0, 840.0],
            );
        }

        crate::debugview::debug_dump_actions::save_flame_history_npy_if_requested(self);
        crate::debugview::debug_dump_actions::save_water_probe_if_requested(self);

        Ok(())
    }
}
