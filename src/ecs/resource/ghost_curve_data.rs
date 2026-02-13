use crate::animation::editable::PropertyType;
use crate::animation::BoneId;
use crate::ml::InferenceRequestId;

#[derive(Clone)]
pub struct GhostCurveSuggestion {
    pub bone_id: BoneId,
    pub property_type: PropertyType,
    pub predicted_time: f32,
    pub predicted_value: f32,
    pub tangent_in: (f32, f32),
    pub tangent_out: (f32, f32),
    pub is_bezier: bool,
    pub confidence: f32,
    pub request_id: InferenceRequestId,
}

pub struct CurveSuggestionState {
    pub suggestions: Vec<GhostCurveSuggestion>,
    pub pending_request_id: Option<InferenceRequestId>,
    pub pending_bone_id: Option<BoneId>,
    pub pending_property_type: Option<PropertyType>,
    pub pending_clip_duration: Option<f32>,
    pub pending_value_scale: Option<f32>,
    pub pending_query_time: Option<f32>,
    pub enabled: bool,
}

impl Default for CurveSuggestionState {
    fn default() -> Self {
        Self {
            suggestions: Vec::new(),
            pending_request_id: None,
            pending_bone_id: None,
            pending_property_type: None,
            pending_clip_duration: None,
            pending_value_scale: None,
            pending_query_time: None,
            enabled: true,
        }
    }
}
