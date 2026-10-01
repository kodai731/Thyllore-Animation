use crate::editable::components::clip::EditableAnimationClip;
use crate::editable::components::morph_track::MorphTrackSample;
use crate::editable::systems::curve_ops::curve_sample;

pub fn sample_morph_tracks(clip: &EditableAnimationClip, time: f32) -> Vec<MorphTrackSample> {
    clip.morph_tracks
        .iter()
        .filter(|track| !track.curve.is_empty())
        .filter_map(|track| {
            curve_sample(&track.curve, time).map(|weight| MorphTrackSample {
                source_mesh: track.source_mesh.clone(),
                channel: track.channel.clone(),
                weight,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editable::systems::curve_ops::curve_add_keyframe;

    #[test]
    fn sample_interpolates_between_two_keys() {
        let mut clip = EditableAnimationClip::new(1, "morph".to_string());
        let track = clip.get_or_add_morph_track("face", "smile");
        curve_add_keyframe(&mut track.curve, 0.0, 0.0);
        curve_add_keyframe(&mut track.curve, 2.0, 1.0);

        let samples = sample_morph_tracks(&clip, 1.0);
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].source_mesh, "face");
        assert_eq!(samples[0].channel, "smile");
        assert!((samples[0].weight - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sample_skips_empty_curves() {
        let mut clip = EditableAnimationClip::new(1, "morph".to_string());
        clip.get_or_add_morph_track("face", "smile");

        let samples = sample_morph_tracks(&clip, 0.0);
        assert!(samples.is_empty());
    }
}
