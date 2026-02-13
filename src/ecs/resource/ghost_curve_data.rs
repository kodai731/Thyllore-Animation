use crate::animation::editable::PropertyType;
use crate::animation::BoneId;
use crate::ml::InferenceRequestId;

#[derive(Clone)]
pub struct GhostCurveSuggestion {
    pub bone_id: BoneId,
    pub property_type: PropertyType,
    pub points: Vec<(f32, f32)>,
    pub confidence: f32,
    pub request_id: InferenceRequestId,
}

pub struct CurveSuggestionState {
    pub suggestions: Vec<GhostCurveSuggestion>,
    pub pending_request_id: Option<InferenceRequestId>,
    pub pending_bone_id: Option<BoneId>,
    pub pending_property_type: Option<PropertyType>,
    pub enabled: bool,
}

impl Default for CurveSuggestionState {
    fn default() -> Self {
        Self {
            suggestions: Vec::new(),
            pending_request_id: None,
            pending_bone_id: None,
            pending_property_type: None,
            enabled: true,
        }
    }
}
