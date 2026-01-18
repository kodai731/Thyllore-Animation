use crate::animation::AnimationClipId;
use crate::ecs::resource::AnimationPlayback;

pub fn playback_play(playback: &mut AnimationPlayback, clip_id: AnimationClipId) {
    playback.current_clip_id = Some(clip_id);
    playback.playing = true;
    playback.time = 0.0;
}

pub fn playback_stop(playback: &mut AnimationPlayback) {
    playback.playing = false;
    playback.current_clip_id = None;
}

pub fn playback_pause(playback: &mut AnimationPlayback) {
    playback.playing = false;
}

pub fn playback_resume(playback: &mut AnimationPlayback) {
    playback.playing = true;
}
