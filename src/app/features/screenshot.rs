use crate::app::App;
use crate::hooks::batch_capture::CaptureSlot;

use anyhow::Result;

impl App {
    pub unsafe fn save_screenshot(&self, image_index: usize) -> Result<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)?
            .as_secs();
        let path = std::path::PathBuf::from(format!("log/screenshot_{}.png", timestamp));
        self.save_screenshot_to(image_index, &path)
    }

    pub unsafe fn save_screenshot_to(
        &self,
        image_index: usize,
        path: &std::path::Path,
    ) -> Result<String> {
        self.capture_context(image_index, CaptureSlot::interactive())
            .save_screenshot_to(path)
    }
}
