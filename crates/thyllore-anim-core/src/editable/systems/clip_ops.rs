use std::collections::HashMap;

use crate::editable::curve_resample_at_fps;
use crate::editable::BoneTrack;
use crate::editable::EditableAnimationClip;
use crate::editable::PropertyType;
use crate::BoneId;

const ALL_PROPERTY_TYPES: [PropertyType; 9] = [
    PropertyType::TranslationX,
    PropertyType::TranslationY,
    PropertyType::TranslationZ,
    PropertyType::RotationX,
    PropertyType::RotationY,
    PropertyType::RotationZ,
    PropertyType::ScaleX,
    PropertyType::ScaleY,
    PropertyType::ScaleZ,
];

#[derive(Clone, Copy, Debug, Default)]
pub struct ResampleSummary {
    pub resampled_curve_count: usize,
    pub total_keyframe_count: usize,
}

pub fn clip_resample_at_fps(clip: &mut EditableAnimationClip, fps: f32) -> ResampleSummary {
    let duration = clip.duration;
    let mut summary = ResampleSummary::default();

    for track in clip.tracks.values_mut() {
        for property_type in ALL_PROPERTY_TYPES {
            let curve = track.get_curve_mut(property_type);
            if curve.is_empty() {
                continue;
            }
            summary.total_keyframe_count += curve_resample_at_fps(curve, duration, fps);
            summary.resampled_curve_count += 1;
        }
    }

    summary
}

pub fn clip_recalculate_duration(clip: &mut EditableAnimationClip) {
    let mut max_time: f32 = 0.0;

    for track in clip.tracks.values().chain(&clip.unbound_tracks) {
        for curve in track.all_curves() {
            if let Some(last_kf) = curve.keyframes.last() {
                max_time = max_time.max(last_kf.time);
            }
        }
    }
    for curve in &clip.scalar_curves {
        if let Some(last_kf) = curve.keyframes.last() {
            max_time = max_time.max(last_kf.time);
        }
    }
    for morph_track in &clip.morph_tracks {
        if let Some(last_kf) = morph_track.curve.keyframes.last() {
            max_time = max_time.max(last_kf.time);
        }
    }

    clip.duration = max_time.max(clip.min_duration);
}

/// Binds every track to the bone of its name; a track whose name the table lacks waits unbound.
pub fn clip_remap_bone_ids(
    clip: &mut EditableAnimationClip,
    name_to_new_id: &HashMap<String, BoneId>,
) {
    let bound_tracks = clip.tracks.drain().map(|(_, track)| track);
    let mut tracks: Vec<BoneTrack> = bound_tracks.chain(clip.unbound_tracks.drain(..)).collect();
    tracks.sort_by(|a, b| a.bone_name.cmp(&b.bone_name));

    for mut track in tracks {
        let free_bone_id = name_to_new_id
            .get(&track.bone_name)
            .copied()
            .filter(|bone_id| !clip.tracks.contains_key(bone_id));
        let Some(bone_id) = free_bone_id else {
            clip.unbound_tracks.push(track);
            continue;
        };

        track.bone_id = bone_id;
        clip.tracks.insert(bone_id, track);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editable::{curve_add_keyframe, curve_sample};

    #[test]
    fn resample_rebakes_curve_onto_60fps_grid() {
        let mut clip = EditableAnimationClip::new(1, "test".to_string());
        let track = clip.add_track(0, "root".to_string());
        let curve = track.get_curve_mut(PropertyType::TranslationX);
        curve_add_keyframe(curve, 0.0, 0.0);
        curve_add_keyframe(curve, 1.0, 10.0);
        clip.duration = 1.0;

        let summary = clip_resample_at_fps(&mut clip, 60.0);

        assert_eq!(summary.resampled_curve_count, 1);
        let curve = clip
            .get_track(0)
            .unwrap()
            .get_curve(PropertyType::TranslationX);
        assert_eq!(curve.keyframes.len(), 61);
        assert!((curve.keyframes.first().unwrap().time - 0.0).abs() < 1e-6);
        assert!((curve.keyframes.last().unwrap().time - 1.0).abs() < 1e-4);
        assert!((curve_sample(curve, 0.5).unwrap() - 5.0).abs() < 0.2);
    }

    #[test]
    fn recalculate_duration_keeps_the_authored_floor_without_keys() {
        let mut clip = EditableAnimationClip::new(1, "unkeyed".to_string());
        clip.min_duration = 12.0;

        clip_recalculate_duration(&mut clip);
        assert!((clip.duration - 12.0).abs() < 1e-6);

        let curve = clip
            .add_track(0, "root".to_string())
            .get_curve_mut(PropertyType::TranslationX);
        curve_add_keyframe(curve, 20.0, 1.0);
        clip_recalculate_duration(&mut clip);
        assert!((clip.duration - 20.0).abs() < 1e-6);
    }

    #[test]
    fn resample_leaves_empty_curves_untouched() {
        let mut clip = EditableAnimationClip::new(1, "test".to_string());
        clip.add_track(0, "root".to_string());
        clip.duration = 1.0;

        let summary = clip_resample_at_fps(&mut clip, 60.0);

        assert_eq!(summary.resampled_curve_count, 0);
        assert!(clip
            .get_track(0)
            .unwrap()
            .get_curve(PropertyType::TranslationX)
            .is_empty());
    }

    fn clip_keyed_on(bone_names: &[&str]) -> EditableAnimationClip {
        let mut clip = EditableAnimationClip::new(1, "authored".to_string());
        for (index, bone_name) in bone_names.iter().enumerate() {
            let curve = clip
                .add_track(index as BoneId, bone_name.to_string())
                .get_curve_mut(PropertyType::RotationX);
            curve_add_keyframe(curve, 2.0 + index as f32, 1.0);
        }
        clip_recalculate_duration(&mut clip);
        clip
    }

    fn name_table(bones: &[(&str, BoneId)]) -> HashMap<String, BoneId> {
        bones
            .iter()
            .map(|(name, bone_id)| (name.to_string(), *bone_id))
            .collect()
    }

    fn bound_names(clip: &EditableAnimationClip) -> Vec<(BoneId, &str)> {
        let mut names: Vec<(BoneId, &str)> = clip
            .tracks
            .iter()
            .map(|(bone_id, track)| (*bone_id, track.bone_name.as_str()))
            .collect();
        names.sort_unstable();
        names
    }

    #[test]
    fn remap_keeps_tracks_of_bones_the_model_lacks_unbound() {
        let mut clip = clip_keyed_on(&["hips", "tail"]);

        clip_remap_bone_ids(&mut clip, &name_table(&[("hips", 7)]));

        assert_eq!(bound_names(&clip), vec![(7, "hips")]);
        assert_eq!(clip.tracks[&7].bone_id, 7);
        assert_eq!(clip.unbound_tracks.len(), 1);
        assert_eq!(clip.unbound_tracks[0].bone_name, "tail");
        assert_eq!(clip.unbound_tracks[0].total_keyframe_count(), 1);
    }

    #[test]
    fn remap_binds_unbound_tracks_once_a_model_has_their_bone() {
        let mut clip = clip_keyed_on(&["hips", "tail"]);
        clip_remap_bone_ids(&mut clip, &name_table(&[("hips", 7)]));

        clip_remap_bone_ids(&mut clip, &name_table(&[("tail", 0), ("hips", 1)]));

        assert_eq!(bound_names(&clip), vec![(0, "tail"), (1, "hips")]);
        assert!(clip.unbound_tracks.is_empty());
        assert_eq!(clip.total_keyframe_count(), 2);
    }

    #[test]
    fn remap_is_idempotent() {
        let mut clip = clip_keyed_on(&["hips", "tail"]);
        let table = name_table(&[("hips", 7)]);
        clip_remap_bone_ids(&mut clip, &table);

        clip_remap_bone_ids(&mut clip, &table);

        assert_eq!(bound_names(&clip), vec![(7, "hips")]);
        assert_eq!(clip.unbound_tracks.len(), 1);
    }

    #[test]
    fn recalculate_duration_includes_unbound_track_keys() {
        let mut clip = clip_keyed_on(&["hips", "tail"]);
        clip_remap_bone_ids(&mut clip, &name_table(&[("hips", 7)]));

        clip_recalculate_duration(&mut clip);

        assert!((clip.duration - 3.0).abs() < 1e-6);
    }

    #[test]
    fn recalculate_duration_includes_morph_track_keys() {
        let mut clip = EditableAnimationClip::new(1, "morph".to_string());
        let track = clip.get_or_add_morph_track("face", "smile");
        curve_add_keyframe(&mut track.curve, 0.0, 0.0);
        curve_add_keyframe(&mut track.curve, 5.0, 1.0);

        clip_recalculate_duration(&mut clip);
        assert!((clip.duration - 5.0).abs() < 1e-6);
    }
}
