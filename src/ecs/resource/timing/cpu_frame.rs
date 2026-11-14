#[derive(Debug, Clone, Default)]
pub struct CpuFrameTimings {
    pub frame: u64,
    pub dt_ms: f32,
    pub compute: ComputeStageTimings,
    pub wait: WaitStageTimings,
    pub imgui_vtx: u32,
    pub imgui_idx: u32,
}

/// Time the CPU spent working, per frame stage, with every GPU wait inside the stage removed.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ComputeStageTimings {
    pub imgui_build_ms: f32,
    pub event_dispatch_ms: f32,
    pub frame_begin_ms: f32,
    pub update_ms: f32,
    pub render_cpu_ms: f32,
    pub last_phase_ms: f32,
}

/// Time the CPU spent blocked on the GPU or the presentation engine.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WaitStageTimings {
    pub gpu_wait_ms: f32,
    pub present_ms: f32,
    pub device_idle_ms: f32,
}
