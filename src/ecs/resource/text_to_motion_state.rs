use crate::animation::editable::EditableAnimationClip;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextToMotionStatus {
    Idle,
    Connecting,
    Generating,
    Generated,
    Error,
}

pub struct TextToMotionState {
    pub status: TextToMotionStatus,
    pub generated_clip: Option<EditableAnimationClip>,
    pub last_prompt: String,
    pub last_duration: f32,
    pub error_message: Option<String>,
    pub generation_time_ms: Option<f32>,
    pub model_used: Option<String>,
    pub server_ready: bool,
}

impl Default for TextToMotionState {
    fn default() -> Self {
        Self {
            status: TextToMotionStatus::Idle,
            generated_clip: None,
            last_prompt: String::new(),
            last_duration: 3.0,
            error_message: None,
            generation_time_ms: None,
            model_used: None,
            server_ready: false,
        }
    }
}
