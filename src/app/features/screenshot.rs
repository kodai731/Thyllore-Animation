use crate::app::App;
use crate::hooks::batch_capture::CaptureSlot;

use anyhow::Result;
use std::path::{Path, PathBuf};

const SCREENSHOT_DIRECTORY: &str = "log";
const SCREENSHOT_FILE_PREFIX: &str = "screenshot_";

pub fn cleanup_old_screenshots() -> Result<()> {
    let log_dir = PathBuf::from(SCREENSHOT_DIRECTORY);
    if !log_dir.exists() {
        return Ok(());
    }

    let mut deleted_count = 0;
    for entry in std::fs::read_dir(&log_dir)? {
        let path = entry?.path();
        if !is_screenshot_file(&path) {
            continue;
        }

        std::fs::remove_file(&path)?;
        deleted_count += 1;
        log!("Deleted old screenshot: {}", path.display());
    }

    if deleted_count > 0 {
        log!("Cleaned up {} old screenshot(s)", deleted_count);
    }

    Ok(())
}

fn is_screenshot_file(path: &Path) -> bool {
    let has_screenshot_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(SCREENSHOT_FILE_PREFIX));

    has_screenshot_name && path.is_file()
}

impl App {
    pub unsafe fn save_screenshot(&self, image_index: usize) -> Result<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)?
            .as_secs();
        let path = PathBuf::from(SCREENSHOT_DIRECTORY)
            .join(format!("{SCREENSHOT_FILE_PREFIX}{timestamp}.png"));
        self.save_screenshot_to(image_index, &path)
    }

    pub unsafe fn save_screenshot_to(&self, image_index: usize, path: &Path) -> Result<String> {
        self.capture_context(image_index, CaptureSlot::interactive())
            .save_screenshot_to(path)
    }
}
