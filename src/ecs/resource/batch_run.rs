use std::path::PathBuf;

use crate::animation::editable::SourceClipId;

#[derive(Clone)]
pub enum BatchRunState {
    WaitingForFrame,
    ScreenshotRequested,
    Completed { result: Result<String, String> },
}

pub struct BatchRun {
    pub output: PathBuf,
    pub screenshot_frame: u64,
    pub frames_rendered: u64,
    pub state: BatchRunState,
    pub captures_remaining: u32,
    pub stride: u32,
    pub sequence_dir: Option<PathBuf>,
    pub total_count: u32,
    pub play_requested: bool,
    pub play_clip_id: Option<SourceClipId>,
}

impl BatchRun {
    pub fn new(output: PathBuf, screenshot_frame: u64) -> Self {
        Self {
            output,
            screenshot_frame,
            frames_rendered: 0,
            state: BatchRunState::WaitingForFrame,
            captures_remaining: 0,
            stride: 1,
            sequence_dir: None,
            total_count: 0,
            play_requested: false,
            play_clip_id: None,
        }
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.state, BatchRunState::Completed { .. })
    }
}
