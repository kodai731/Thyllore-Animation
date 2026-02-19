pub struct TextToMotionRequest {
    pub prompt: String,
    pub duration_seconds: f32,
    pub target_fps: i32,
}

pub enum GrpcRequest {
    GenerateMotion(TextToMotionRequest),
    CheckStatus,
    Shutdown,
}

pub enum GrpcResponse {
    MotionGenerated {
        curves: Vec<RawAnimationCurve>,
        generation_time_ms: f32,
        model_used: String,
    },
    ServerStatus {
        ready: bool,
        active_model: String,
        gpu_memory_mb: i32,
    },
    Error {
        message: String,
    },
}

pub struct RawAnimationCurve {
    pub bone_name: String,
    pub property_type: i32,
    pub keyframes: Vec<RawCurveKeyframe>,
}

pub struct RawCurveKeyframe {
    pub time: f32,
    pub value: f32,
    pub tangent_in_dt: f32,
    pub tangent_in_dv: f32,
    pub tangent_out_dt: f32,
    pub tangent_out_dv: f32,
    pub interpolation: i32,
}
