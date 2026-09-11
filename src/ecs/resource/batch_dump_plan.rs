use std::path::PathBuf;

#[derive(Default, Clone, Debug)]
pub struct BatchDumpPlan {
    pub dump_wall_probe: bool,
    pub dump_water_debug: bool,
    pub flame_set: Vec<(String, f32)>,
    pub flame_trace_path: Option<PathBuf>,
    pub wall_probe_path: Option<PathBuf>,
    pub water_probe_path: Option<PathBuf>,
}
