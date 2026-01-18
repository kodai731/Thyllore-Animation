use crate::animation::AnimationClipId;

#[derive(Clone, Debug)]
pub struct AnimationPlayback {
    pub time: f32,
    pub playing: bool,
    pub current_index: usize,
    pub model_path: String,
    pub current_clip_id: Option<AnimationClipId>,
    pub speed: f32,
    pub looping: bool,
}

impl AnimationPlayback {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            playing: true,
            current_index: 0,
            model_path: String::new(),
            current_clip_id: None,
            speed: 1.0,
            looping: true,
        }
    }

    pub fn with_model_path(model_path: String) -> Self {
        Self {
            time: 0.0,
            playing: true,
            current_index: 0,
            model_path,
            current_clip_id: None,
            speed: 1.0,
            looping: true,
        }
    }
}

impl Default for AnimationPlayback {
    fn default() -> Self {
        Self::new()
    }
}
