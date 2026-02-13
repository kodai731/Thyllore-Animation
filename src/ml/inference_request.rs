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
        points: Vec<(f32, f32)>,
        confidence: f32,
    },
}

#[derive(Clone, Debug)]
pub struct InferenceResult {
    pub request_id: InferenceRequestId,
    pub actor_id: InferenceActorId,
    pub kind: InferenceResultKind,
}
