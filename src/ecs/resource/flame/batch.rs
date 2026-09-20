use std::path::PathBuf;

/// Requests the wall probe and field traces under `log/` at the capture frame (`dump_wall_probe`).
#[derive(Default, Clone, Debug, PartialEq)]
pub struct FlameWallProbeCapture;

/// Requests the same traces at the paths `--batch-flame-trace` / `--batch-wall-probe` gave.
#[derive(Clone, Debug, PartialEq)]
pub struct FlameFieldTraceCapture {
    pub trace_path: Option<PathBuf>,
    pub wall_probe_path: Option<PathBuf>,
}
