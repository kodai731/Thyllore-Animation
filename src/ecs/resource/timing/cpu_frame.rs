use serde::{Deserialize, Serialize};

pub const STAGE_IMGUI_BUILD: &str = "imgui_build";
pub const STAGE_EVENT_DISPATCH: &str = "event_dispatch";
pub const STAGE_GPU_WAIT: &str = "gpu_wait";
pub const STAGE_UPDATE: &str = "update";
pub const STAGE_RENDER_CPU: &str = "render_cpu";
pub const STAGE_PRESENT: &str = "present";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuFrameTimings {
    pub frame: u64,
    pub dt_ms: f32,
    pub stages: Vec<(String, f32)>,
    pub imgui_vtx: u32,
    pub imgui_idx: u32,
}

fn get_stage(timings: &CpuFrameTimings, name: &str) -> f32 {
    timings
        .stages
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| *v)
        .unwrap_or(0.0)
}

pub fn cpu_ms(timings: &CpuFrameTimings) -> f32 {
    get_stage(timings, STAGE_IMGUI_BUILD)
        + get_stage(timings, STAGE_EVENT_DISPATCH)
        + get_stage(timings, STAGE_UPDATE)
        + get_stage(timings, STAGE_RENDER_CPU)
}

pub fn wait_ms(timings: &CpuFrameTimings) -> f32 {
    get_stage(timings, STAGE_GPU_WAIT) + get_stage(timings, STAGE_PRESENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timings(stages: &[(&str, f32)]) -> CpuFrameTimings {
        CpuFrameTimings {
            frame: 0,
            dt_ms: 16.0,
            stages: stages.iter().map(|(k, v)| ((*k).to_string(), *v)).collect(),
            imgui_vtx: 0,
            imgui_idx: 0,
        }
    }

    #[test]
    fn test_cpu_ms() {
        let t = timings(&[
            (STAGE_IMGUI_BUILD, 1.0),
            (STAGE_EVENT_DISPATCH, 0.5),
            (STAGE_UPDATE, 2.0),
            (STAGE_RENDER_CPU, 3.0),
        ]);
        assert!((cpu_ms(&t) - 6.5).abs() < 1e-6);
    }

    #[test]
    fn test_cpu_ms_missing_stages() {
        let t = timings(&[(STAGE_IMGUI_BUILD, 1.0), (STAGE_UPDATE, 2.0)]);
        assert!((cpu_ms(&t) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_cpu_ms_empty() {
        let t = timings(&[] as &[(&str, f32)]);
        assert_eq!(cpu_ms(&t), 0.0);
    }

    #[test]
    fn test_wait_ms() {
        let t = timings(&[(STAGE_GPU_WAIT, 1.0), (STAGE_PRESENT, 0.5)]);
        assert!((wait_ms(&t) - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_wait_ms_missing_stages() {
        let t = timings(&[(STAGE_GPU_WAIT, 1.0)]);
        assert!((wait_ms(&t) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_wait_ms_empty() {
        let t = timings(&[] as &[(&str, f32)]);
        assert_eq!(wait_ms(&t), 0.0);
    }
}
