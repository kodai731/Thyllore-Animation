use std::collections::HashMap;

use crate::animation::editable::{
    clip_add_keyframe, clip_recalculate_duration, EditableAnimationClip, PropertyType,
};
use crate::animation::{BoneId, BoneLocalPose, Skeleton};
use crate::ecs::resource::{ClipLibrary, HumanoidRig, TimelineState};
use crate::ecs::systems::humanoid_import_systems::{local_pose_to_standard, StandardBoneKey};

pub fn process_bone_set_key(
    overrides: &HashMap<BoneId, BoneLocalPose>,
    clip_library: &mut ClipLibrary,
    timeline_state: &TimelineState,
    skeleton: &Skeleton,
    rig: Option<&HumanoidRig>,
) -> bool {
    let Some(clip_id) = timeline_state.current_clip_id else {
        return false;
    };
    let Some(clip) = clip_library.get_mut(clip_id) else {
        return false;
    };
    if overrides.is_empty() {
        return false;
    }

    let time = timeline_state.current_time;
    for (&bone_id, local_pose) in overrides {
        let standard = rig.and_then(|rig| {
            local_pose_to_standard(
                rig,
                skeleton,
                bone_id,
                local_pose.rotation,
                local_pose.translation,
            )
        });
        match standard {
            Some(standard) => key_standard_pose(clip, bone_id, time, &standard),
            None => {
                let bone_name = raw_track_name(rig, skeleton, bone_id);
                key_local_pose(clip, bone_id, bone_name, time, local_pose);
            }
        }
    }

    clip_recalculate_duration(clip);
    true
}

fn key_standard_pose(
    clip: &mut EditableAnimationClip,
    bone_id: BoneId,
    time: f32,
    standard: &StandardBoneKey,
) {
    let track_name = standard.role.unity_name().to_string();
    match clip.tracks.get_mut(&bone_id) {
        Some(track) => track.bone_name = track_name,
        None => {
            clip.add_track(bone_id, track_name);
        }
    }

    let [x_degrees, y_degrees, z_degrees] = standard.euler_degrees;
    clip_add_keyframe(clip, bone_id, PropertyType::RotationX, time, x_degrees);
    clip_add_keyframe(clip, bone_id, PropertyType::RotationY, time, y_degrees);
    clip_add_keyframe(clip, bone_id, PropertyType::RotationZ, time, z_degrees);

    if let Some([x, y, z]) = standard.hips_translation {
        clip_add_keyframe(clip, bone_id, PropertyType::TranslationX, time, x);
        clip_add_keyframe(clip, bone_id, PropertyType::TranslationY, time, y);
        clip_add_keyframe(clip, bone_id, PropertyType::TranslationZ, time, z);
    }
}

fn raw_track_name(rig: Option<&HumanoidRig>, skeleton: &Skeleton, bone_id: BoneId) -> String {
    rig.and_then(|r| r.track_names.get(&bone_id).cloned())
        .or_else(|| skeleton.get_bone(bone_id).map(|b| b.name.clone()))
        .unwrap_or_else(|| format!("bone_{}", bone_id))
}

fn key_local_pose(
    clip: &mut EditableAnimationClip,
    bone_id: BoneId,
    bone_name: String,
    time: f32,
    local_pose: &BoneLocalPose,
) {
    if !clip.tracks.contains_key(&bone_id) {
        clip.add_track(bone_id, bone_name);
    }

    let euler = crate::math::quaternion_to_euler_degrees(&local_pose.rotation);
    let t = &local_pose.translation;
    let s = &local_pose.scale;
    clip_add_keyframe(clip, bone_id, PropertyType::TranslationX, time, t.x);
    clip_add_keyframe(clip, bone_id, PropertyType::TranslationY, time, t.y);
    clip_add_keyframe(clip, bone_id, PropertyType::TranslationZ, time, t.z);
    clip_add_keyframe(clip, bone_id, PropertyType::RotationX, time, euler.x);
    clip_add_keyframe(clip, bone_id, PropertyType::RotationY, time, euler.y);
    clip_add_keyframe(clip, bone_id, PropertyType::RotationZ, time, euler.z);
    clip_add_keyframe(clip, bone_id, PropertyType::ScaleX, time, s.x);
    clip_add_keyframe(clip, bone_id, PropertyType::ScaleY, time, s.y);
    clip_add_keyframe(clip, bone_id, PropertyType::ScaleZ, time, s.z);
}
