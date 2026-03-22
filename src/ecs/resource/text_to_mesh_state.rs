#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextToMeshStatus {
    Idle,
    Generating,
    Generated,
    Error,
}

pub struct TextToMeshState {
    pub status: TextToMeshStatus,
    pub glb_data: Option<Vec<u8>>,
    pub last_prompt: String,
    pub error_message: Option<String>,
    pub generation_time_ms: Option<f32>,
    pub vertex_count: Option<u32>,
    pub face_count: Option<u32>,
    pub intermediate_image_png: Option<Vec<u8>>,
}

impl Default for TextToMeshState {
    fn default() -> Self {
        Self {
            status: TextToMeshStatus::Idle,
            glb_data: None,
            last_prompt: String::new(),
            error_message: None,
            generation_time_ms: None,
            vertex_count: None,
            face_count: None,
            intermediate_image_png: None,
        }
    }
}
