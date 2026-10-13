use crate::grpc::{MeshInputMode, MeshModelType, TextToImageModelType};

pub struct TextToMeshDialogState {
    pub open: bool,
    pub prompt_buf: String,
    pub target_faces: i32,
    pub seed: i32,
    pub generate_start_time: Option<std::time::Instant>,
    pub input_mode: MeshInputMode,
    pub model_type: MeshModelType,
    pub t2i_model_type: TextToImageModelType,
    pub image_path: String,
    pub image_bytes: Option<Vec<u8>>,
    pub image_load_error: Option<String>,
}

impl Default for TextToMeshDialogState {
    fn default() -> Self {
        Self {
            open: false,
            prompt_buf: String::new(),
            target_faces: 50000,
            seed: 0,
            generate_start_time: None,
            input_mode: MeshInputMode::TextOnly,
            model_type: MeshModelType::Trellis,
            t2i_model_type: TextToImageModelType::ServerDefault,
            image_path: String::new(),
            image_bytes: None,
            image_load_error: None,
        }
    }
}

pub struct TextToAnimationDialogState {
    pub open: bool,
    pub prompt_buf: String,
    pub duration: f32,
}

impl Default for TextToAnimationDialogState {
    fn default() -> Self {
        Self {
            open: false,
            prompt_buf: String::new(),
            duration: 3.0,
        }
    }
}
