pub fn resolve_curve_copilot_model_path() -> String {
    let shared_path = std::path::Path::new(crate::paths::CURVE_COPILOT_MODEL);
    if shared_path.exists() {
        crate::log!("Using SharedData model: {}", crate::paths::CURVE_COPILOT_MODEL);
        return crate::paths::CURVE_COPILOT_MODEL.to_string();
    }

    if let Some(latest) = find_latest_curve_copilot_model() {
        crate::log!("Using dated SharedData model: {}", latest);
        return latest;
    }

    crate::log!("SharedData model not found, falling back to dummy model");
    crate::paths::CURVE_COPILOT_DUMMY_MODEL.to_string()
}

fn find_latest_curve_copilot_model() -> Option<String> {
    let exports_dir = std::path::Path::new(crate::paths::SHARED_EXPORTS_DIR);
    let entries = std::fs::read_dir(exports_dir).ok()?;

    entries
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("curve_copilot_") && name.ends_with(".onnx") {
                Some(entry.path().to_string_lossy().to_string())
            } else {
                None
            }
        })
        .max()
}

pub type InferenceActorId = u64;
pub type InferenceRequestId = u64;

pub const CURVE_COPILOT_ACTOR_ID: InferenceActorId = 2;

#[derive(Clone, Debug)]
pub enum InferenceModelKind {
    CurvePredictor,
    CurveCopilot,
}

#[derive(Clone, Debug)]
pub enum InferenceRequestKind {
    CurvePredict { input: Vec<f32> },
    CurveCopilotPredict {
        context: Vec<f32>,
        property_type_id: u32,
        topology_features: Vec<f32>,
        bone_name_tokens: Vec<i64>,
        query_time: f32,
    },
}

#[derive(Clone, Debug)]
pub struct InferenceRequest {
    pub request_id: InferenceRequestId,
    pub actor_id: InferenceActorId,
    pub kind: InferenceRequestKind,
}

#[derive(Clone, Debug)]
pub enum InferenceResultKind {
    CurvePredict { output: Vec<f32> },
    CurveCopilotPredict {
        value: f32,
        tangent_in: (f32, f32),
        tangent_out: (f32, f32),
        is_bezier: bool,
        confidence: f32,
    },
}

#[derive(Clone, Debug)]
pub struct InferenceResult {
    pub request_id: InferenceRequestId,
    pub actor_id: InferenceActorId,
    pub kind: InferenceResultKind,
}
