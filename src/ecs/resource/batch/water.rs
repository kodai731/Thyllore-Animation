use std::path::PathBuf;

/// Requests the water debug dump under `log/water/` at the capture frame (`dump_water_debug`).
#[derive(Default, Clone, Debug, PartialEq)]
pub struct WaterDebugCapture;

/// Requests the HDR image comparison against the analytic torus (`--batch-water-probe <path>`).
#[derive(Clone, Debug, PartialEq)]
pub struct WaterProbeCapture {
    pub path: PathBuf,
}
