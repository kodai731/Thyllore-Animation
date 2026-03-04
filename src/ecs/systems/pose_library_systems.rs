use crate::animation::editable::{
    quaternion_to_euler_degrees, EditableAnimationClip, PropertyCurve, SourceClipId,
};
use crate::asset::AssetStorage;
use crate::ecs::resource::ClipLibrary;
use crate::ecs::systems::skeleton_pose_systems::{create_pose_from_rest, sample_clip_to_pose};

pub fn capture_current_pose(
    name: &str,
    clip_library: &ClipLibrary,
    assets: &AssetStorage,
    current_clip_id: Option<SourceClipId>,
    current_time: f32,
) -> Option<EditableAnimationClip> {
    let clip_id = current_clip_id?;
    let asset_id = clip_library.get_asset_id_for_source(clip_id)?;
    let clip_asset = assets.animation_clips.get(&asset_id)?;
    let skeleton_asset = assets.skeletons.values().next()?;
    let skeleton = &skeleton_asset.skeleton;

    let mut pose = create_pose_from_rest(skeleton);
    sample_clip_to_pose(&clip_asset.clip, current_time, skeleton, &mut pose, false);

    let mut editable = EditableAnimationClip::new(0, name.to_string());
    editable.duration = 0.0;

    for bone in &skeleton.bones {
        let idx = bone.id as usize;
        if idx >= pose.bone_poses.len() {
            continue;
        }

        let bp = &pose.bone_poses[idx];
        let track = editable.add_track(bone.id, bone.name.clone());

        track.translation_x.add_keyframe(0.0, bp.translation.x);
        track.translation_y.add_keyframe(0.0, bp.translation.y);
        track.translation_z.add_keyframe(0.0, bp.translation.z);

        let euler = quaternion_to_euler_degrees(&bp.rotation);
        track.rotation_x.add_keyframe(0.0, euler.x);
        track.rotation_y.add_keyframe(0.0, euler.y);
        track.rotation_z.add_keyframe(0.0, euler.z);

        track.scale_x.add_keyframe(0.0, bp.scale.x);
        track.scale_y.add_keyframe(0.0, bp.scale.y);
        track.scale_z.add_keyframe(0.0, bp.scale.z);
    }

    Some(editable)
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
    source_curve: &PropertyCurve,
    target_curve: &mut PropertyCurve,
    target_time: f32,
) {
    if let Some(kf) = source_curve.keyframes.first() {
        target_curve.add_keyframe(target_time, kf.value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_capture_without_clip_returns_none() {
        let clip_library = ClipLibrary::default();
        let assets = AssetStorage::default();
        let result = capture_current_pose("Test", &clip_library, &assets, None, 0.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_inserts_keyframes_at_target_time() {
        let mut pose_clip = EditableAnimationClip::new(0, "Pose".to_string());
        pose_clip.duration = 0.0;
        let track = pose_clip.add_track(0, "Bone0".to_string());
        track.translation_x.add_keyframe(0.0, 1.5);
        track.translation_y.add_keyframe(0.0, 2.0);
        track.translation_z.add_keyframe(0.0, 3.0);
        track.rotation_x.add_keyframe(0.0, 10.0);
        track.rotation_y.add_keyframe(0.0, 20.0);
        track.rotation_z.add_keyframe(0.0, 30.0);
        track.scale_x.add_keyframe(0.0, 1.0);
        track.scale_y.add_keyframe(0.0, 1.0);
        track.scale_z.add_keyframe(0.0, 1.0);

        let mut target = EditableAnimationClip::new(1, "Target".to_string());
        target.duration = 5.0;
        let t_track = target.add_track(0, "Bone0".to_string());
        t_track.translation_x.add_keyframe(0.0, 0.0);

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
        track.translation_x.add_keyframe(0.0, 7.0);

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
        track.translation_x.add_keyframe(0.0, 1.0);

        let mut target = EditableAnimationClip::new(1, "Target".to_string());
        target.duration = 1.0;

        apply_pose_to_clip(&pose_clip, &mut target, 3.0);
        assert!((target.duration - 3.0).abs() < 0.001);
    }
}
