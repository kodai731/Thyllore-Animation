use crate::animation::editable::{
    curve_add_keyframe, curve_sample, EditableAnimationClip, PropertyType, SourceClipId,
};
use crate::ecs::resource::ClipLibrary;

const ALL_PROPERTIES: [PropertyType; 9] = [
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

pub fn capture_current_pose(
    name: &str,
    clip_library: &ClipLibrary,
    current_clip_id: Option<SourceClipId>,
    current_time: f32,
) -> Option<EditableAnimationClip> {
    let clip_id = current_clip_id?;
    let source_clip = clip_library.get(clip_id)?;

    let mut pose_clip = EditableAnimationClip::new(0, name.to_string());
    pose_clip.duration = 0.0;

    for (&bone_id, source_track) in &source_clip.tracks {
        let pose_track = pose_clip.add_track(bone_id, source_track.bone_name.clone());

        for &prop in &ALL_PROPERTIES {
            let source_curve = source_track.get_curve(prop);
            if let Some(value) = curve_sample(source_curve, current_time) {
                let pose_curve = pose_track.get_curve_mut(prop);
                curve_add_keyframe(pose_curve, 0.0, value);
            }
        }
    }

    Some(pose_clip)
}

pub fn apply_pose_to_clip(
    pose_clip: &EditableAnimationClip,
    target_clip: &mut EditableAnimationClip,
    target_time: f32,
) {
    for (&bone_id, pose_track) in &pose_clip.tracks {
        let target_track = if target_clip.tracks.contains_key(&bone_id) {
            target_clip.tracks.get_mut(&bone_id).unwrap()
        } else {
            target_clip.add_track(bone_id, pose_track.bone_name.clone())
        };

        insert_keyframe_from_pose(
            &pose_track.translation_x,
            &mut target_track.translation_x,
            target_time,
        );
        insert_keyframe_from_pose(
            &pose_track.translation_y,
            &mut target_track.translation_y,
            target_time,
        );
        insert_keyframe_from_pose(
            &pose_track.translation_z,
            &mut target_track.translation_z,
            target_time,
        );
        insert_keyframe_from_pose(
            &pose_track.rotation_x,
            &mut target_track.rotation_x,
            target_time,
        );
        insert_keyframe_from_pose(
            &pose_track.rotation_y,
            &mut target_track.rotation_y,
            target_time,
        );
        insert_keyframe_from_pose(
            &pose_track.rotation_z,
            &mut target_track.rotation_z,
            target_time,
        );
        insert_keyframe_from_pose(&pose_track.scale_x, &mut target_track.scale_x, target_time);
        insert_keyframe_from_pose(&pose_track.scale_y, &mut target_track.scale_y, target_time);
        insert_keyframe_from_pose(&pose_track.scale_z, &mut target_track.scale_z, target_time);
    }

    target_clip.recalculate_duration();
}

fn insert_keyframe_from_pose(
    source_curve: &crate::animation::editable::PropertyCurve,
    target_curve: &mut crate::animation::editable::PropertyCurve,
    target_time: f32,
) {
    if let Some(kf) = source_curve.keyframes.first() {
        curve_add_keyframe(target_curve, target_time, kf.value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_capture_without_clip_returns_none() {
        let clip_library = ClipLibrary::default();
        let result = capture_current_pose("Test", &clip_library, None, 0.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_inserts_keyframes_at_target_time() {
        let mut pose_clip = EditableAnimationClip::new(0, "Pose".to_string());
        pose_clip.duration = 0.0;
        let track = pose_clip.add_track(0, "Bone0".to_string());
        curve_add_keyframe(&mut track.translation_x, 0.0, 1.5);
        curve_add_keyframe(&mut track.translation_y, 0.0, 2.0);
        curve_add_keyframe(&mut track.translation_z, 0.0, 3.0);
        curve_add_keyframe(&mut track.rotation_x, 0.0, 10.0);
        curve_add_keyframe(&mut track.rotation_y, 0.0, 20.0);
        curve_add_keyframe(&mut track.rotation_z, 0.0, 30.0);
        curve_add_keyframe(&mut track.scale_x, 0.0, 1.0);
        curve_add_keyframe(&mut track.scale_y, 0.0, 1.0);
        curve_add_keyframe(&mut track.scale_z, 0.0, 1.0);

        let mut target = EditableAnimationClip::new(1, "Target".to_string());
        target.duration = 5.0;
        let t_track = target.add_track(0, "Bone0".to_string());
        curve_add_keyframe(&mut t_track.translation_x, 0.0, 0.0);

        apply_pose_to_clip(&pose_clip, &mut target, 2.5);

        let result_track = target.get_track(0).unwrap();
        assert_eq!(result_track.translation_x.keyframe_count(), 2);

        let kf = result_track
            .translation_x
            .keyframes
            .iter()
            .find(|k| (k.time - 2.5).abs() < 0.001);
        assert!(kf.is_some());
        assert!((kf.unwrap().value - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_apply_creates_track_if_missing() {
        let mut pose_clip = EditableAnimationClip::new(0, "Pose".to_string());
        let track = pose_clip.add_track(5, "NewBone".to_string());
        curve_add_keyframe(&mut track.translation_x, 0.0, 7.0);

        let mut target = EditableAnimationClip::new(1, "Target".to_string());
        assert!(target.get_track(5).is_none());

        apply_pose_to_clip(&pose_clip, &mut target, 1.0);

        let result_track = target.get_track(5).unwrap();
        assert_eq!(result_track.bone_name, "NewBone");
        assert_eq!(result_track.translation_x.keyframe_count(), 1);
        let kf = &result_track.translation_x.keyframes[0];
        assert!((kf.time - 1.0).abs() < 0.001);
        assert!((kf.value - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_apply_recalculates_duration() {
        let mut pose_clip = EditableAnimationClip::new(0, "Pose".to_string());
        let track = pose_clip.add_track(0, "Bone".to_string());
        curve_add_keyframe(&mut track.translation_x, 0.0, 1.0);

        let mut target = EditableAnimationClip::new(1, "Target".to_string());
        target.duration = 1.0;

        apply_pose_to_clip(&pose_clip, &mut target, 3.0);
        assert!((target.duration - 3.0).abs() < 0.001);
    }
}
