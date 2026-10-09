use std::path::{Path, PathBuf};

use crate::animation::editable::SourceClipId;

/// Where the captures of a batch run go: one PNG, or `frame_XX.png` files under a directory.
#[derive(Clone, Debug, PartialEq)]
pub enum CaptureOutput {
    Single(PathBuf),
    Sequence(PathBuf),
}

impl CaptureOutput {
    pub fn sequence_dir(&self) -> Option<&Path> {
        match self {
            CaptureOutput::Single(_) => None,
            CaptureOutput::Sequence(dir) => Some(dir),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaptureSchedule {
    pub first_frame: u64,
    pub stride: u32,
    pub count: u32,
    pub output: CaptureOutput,
}

impl CaptureSchedule {
    pub fn single(output: PathBuf, first_frame: u64) -> Self {
        Self {
            first_frame,
            stride: 1,
            count: 1,
            output: CaptureOutput::Single(output),
        }
    }

    pub fn frame_of_capture(&self, index: u32) -> u64 {
        self.first_frame + self.stride as u64 * index as u64
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BatchRunState {
    WaitingForFrame { next_frame: u64, captured: u32 },
    CaptureRequested { index: u32 },
    Completed { result: Result<String, String> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct BatchPlayback {
    pub clip_id: Option<SourceClipId>,
}

/// A headless run: what to capture, how far it got, and whether playback was requested.
/// The frame it is measured against is `FrameClock`.
#[derive(Clone, Debug)]
pub struct BatchRun {
    pub capture: CaptureSchedule,
    pub state: BatchRunState,
    pub playback: Option<BatchPlayback>,
}

impl BatchRun {
    pub fn new(capture: CaptureSchedule) -> Self {
        Self {
            state: BatchRunState::WaitingForFrame {
                next_frame: capture.first_frame,
                captured: 0,
            },
            capture,
            playback: None,
        }
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.state, BatchRunState::Completed { .. })
    }
}
