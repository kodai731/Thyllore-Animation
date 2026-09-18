use std::path::PathBuf;

/// What the flame effect writes at a batch capture frame.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct FlameBatchCapture {
    pub wall_probe: bool,
    pub trace_path: Option<PathBuf>,
    pub wall_probe_path: Option<PathBuf>,
}
