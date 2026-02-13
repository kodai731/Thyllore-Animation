pub type InferenceActorId = u64;
pub type InferenceRequestId = u64;

#[derive(Clone, Debug)]
pub enum InferenceModelKind {
    CurvePredictor,
}

#[derive(Clone, Debug)]
pub enum InferenceRequestKind {
    CurvePredict { input: Vec<f32> },
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
}

#[derive(Clone, Debug)]
pub struct InferenceResult {
    pub request_id: InferenceRequestId,
    pub actor_id: InferenceActorId,
    pub kind: InferenceResultKind,
}
