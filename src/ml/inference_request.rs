pub fn resolve_curve_copilot_model_path() -> String {
    let shared_path = std::path::Path::new(crate::paths::CURVE_COPILOT_MODEL);
    if shared_path.exists() {
        crate::log!("Using SharedData model: {}", crate::paths::CURVE_COPILOT_MODEL);
        crate::paths::CURVE_COPILOT_MODEL.to_string()
    } else {
        crate::log!("SharedData model not found, falling back to dummy model");
        crate::paths::CURVE_COPILOT_DUMMY_MODEL.to_string()
    }
}

pub type InferenceActorId = u64;
pub type InferenceRequestId = u64;

pub const CURVE_COPILOT_ACTOR_ID: InferenceActorId = 2;

#[derive(Clone, Debug)]
pub enum InferenceModelKind {
    CurvePredictor,
    CurveCopilot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointCategory {
    Root = 0,
    Spine = 1,
    Arm = 2,
    Leg = 3,
    Head = 4,
    Hand = 5,
    Foot = 6,
}

pub fn classify_bone_name(name: &str) -> JointCategory {
    let lower = name.to_lowercase();

    if lower.contains("hand") || lower.contains("wrist") {
        return JointCategory::Hand;
    }
    if lower.contains("foot") || lower.contains("ankle") || lower.contains("toe") {
        return JointCategory::Foot;
    }
    if lower.contains("head") || lower.contains("neck") || lower.contains("jaw") {
        return JointCategory::Head;
    }
    if lower.contains("arm") || lower.contains("shoulder") || lower.contains("elbow") {
        return JointCategory::Arm;
    }
    if lower.contains("leg")
        || lower.contains("hip")
        || lower.contains("knee")
        || lower.contains("thigh")
        || lower.contains("calf")
        || lower.contains("shin")
        || lower.contains("upleg")
    {
        return JointCategory::Leg;
    }
    if lower.contains("spine")
        || lower.contains("chest")
        || lower.contains("torso")
        || lower.contains("pelvis")
        || lower.contains("waist")
    {
        return JointCategory::Spine;
    }
    if lower.contains("root") || lower.contains("hips") {
        return JointCategory::Root;
    }

    JointCategory::Spine
}

#[derive(Clone, Debug)]
pub enum InferenceRequestKind {
    CurvePredict { input: Vec<f32> },
    CurveCopilotPredict {
        context: Vec<f32>,
        property_type_id: u32,
        joint_category: u32,
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
