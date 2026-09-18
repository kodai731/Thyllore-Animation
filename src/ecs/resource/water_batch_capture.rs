use std::path::PathBuf;

/// What the water effect writes at a batch capture frame.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct WaterBatchCapture {
    pub debug_dump: bool,
    pub probe_path: Option<PathBuf>,
}
