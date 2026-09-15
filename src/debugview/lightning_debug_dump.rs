use crate::app::App;
use crate::ecs::systems::{
    build_lightning_debug_record, current_unix_time, lightning_debug_screenshot_path,
    write_lightning_debug_dump,
};

impl App {
    pub fn dump_lightning_debug(&self) {
        self.dump_lightning_debug_at(self.frame % crate::app::init::MAX_FRAMES_IN_FLIGHT);
    }

    pub fn dump_lightning_debug_at(&self, image_index: usize) {
        let unix_time = current_unix_time();

        let screenshot_path = lightning_debug_screenshot_path(unix_time);
        let saved_screenshot =
            match unsafe { self.save_screenshot_to(image_index, &screenshot_path) } {
                Ok(_) => Some(screenshot_path.as_path()),
                Err(error) => {
                    log_warn!("lightning debug screenshot failed: {:?}", error);
                    None
                }
            };

        let record =
            build_lightning_debug_record(&self.data.ecs_world, saved_screenshot, unix_time);
        match write_lightning_debug_dump(&record, unix_time) {
            Ok(path) => msg_info!("Lightning debug dumped: {}", path.display()),
            Err(error) => log_error!("lightning debug dump failed: {}", error),
        }
    }
}
