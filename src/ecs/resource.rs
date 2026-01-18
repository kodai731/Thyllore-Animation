use crate::scene::animation::AnimationClipId;

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

    pub fn play(&mut self, clip_id: AnimationClipId) {
        self.current_clip_id = Some(clip_id);
        self.playing = true;
        self.time = 0.0;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.current_clip_id = None;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn resume(&mut self) {
        self.playing = true;
    }

    pub fn update(&mut self, delta_time: f32, clip_duration: f32) {
        if !self.playing || clip_duration <= 0.0 {
            return;
        }

        self.time += delta_time * self.speed;

        if self.looping {
            self.time = self.time % clip_duration;
        } else if self.time >= clip_duration {
            self.time = clip_duration;
            self.playing = false;
        }
    }
}

impl Default for AnimationPlayback {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct ModelInfo {
    pub has_skinned_meshes: bool,
    pub node_animation_scale: f32,
}

impl ModelInfo {
    pub fn new() -> Self {
        Self {
            has_skinned_meshes: false,
            node_animation_scale: 1.0,
        }
    }
}

impl Default for ModelInfo {
    fn default() -> Self {
        Self::new()
    }
}
