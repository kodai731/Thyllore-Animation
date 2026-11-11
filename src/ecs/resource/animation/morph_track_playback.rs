#[derive(Default)]
pub struct MorphTrackPlayback {
    pub last_time: Option<f32>,
}

crate::startup_resource!(MorphTrackPlayback, Editor);
