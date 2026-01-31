#[derive(Clone, Debug)]
pub struct AnimationPlayback {
    pub time: f32,
    pub playing: bool,
    pub speed: f32,
    pub looping: bool,
}

impl AnimationPlayback {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            playing: true,
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
