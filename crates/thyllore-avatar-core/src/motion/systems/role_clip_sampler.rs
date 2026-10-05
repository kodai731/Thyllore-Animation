use std::collections::BTreeMap;

use thyllore_anim_core::editable::components::clip::EditableAnimationClip;
use thyllore_anim_core::editable::systems::curve_ops::curve_sample;
use thyllore_anim_core::editable::systems::morph_sample::sample_morph_tracks;

use crate::humanoid::components::role::HumanoidRole;
use crate::motion::components::sampled_pose::SampledPose;

pub fn sample_role_clip(clip: &EditableAnimationClip, time: f32) -> SampledPose {
    let mut rotations: BTreeMap<HumanoidRole, [f32; 3]> = BTreeMap::new();
    let mut hips_translation = [0.0f32; 3];

    for track in clip.tracks.values() {
        let bone_id = track.bone_id as usize;
        if bone_id >= HumanoidRole::ALL.len() {
            continue;
        }
        let role = HumanoidRole::ALL[bone_id];

        let rx = curve_sample(&track.rotation_x, time).unwrap_or(0.0);
        let ry = curve_sample(&track.rotation_y, time).unwrap_or(0.0);
        let rz = curve_sample(&track.rotation_z, time).unwrap_or(0.0);

        if !track.rotation_x.is_empty()
            || !track.rotation_y.is_empty()
            || !track.rotation_z.is_empty()
        {
            rotations.insert(role, [rx, ry, rz]);
        }

        if role == HumanoidRole::Hips {
            hips_translation[0] = curve_sample(&track.translation_x, time).unwrap_or(0.0);
            hips_translation[1] = curve_sample(&track.translation_y, time).unwrap_or(0.0);
            hips_translation[2] = curve_sample(&track.translation_z, time).unwrap_or(0.0);
        }
    }

    let morph: BTreeMap<String, f32> = sample_morph_tracks(clip, time)
        .into_iter()
        .map(|s| (s.channel, s.weight))
        .collect();

    SampledPose {
        rotations,
        hips_translation,
        morph,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_anim_core::editable::systems::curve_ops::curve_add_keyframe;

    fn make_clip_with_track(bone_id: u32) -> EditableAnimationClip {
        let mut clip = EditableAnimationClip::new(1, "test".to_string());
        clip.add_track(bone_id, format!("bone_{bone_id}"));
        clip
    }

    #[test]
    fn sample_role_clip_interpolates_rotation() {
        let bone_id = HumanoidRole::ALL
            .iter()
            .position(|r| *r == HumanoidRole::LeftUpperArm);
        let Some(bone_id) = bone_id else {
            panic!("LeftUpperArm not found in HumanoidRole::ALL");
        };

        let mut clip = make_clip_with_track(bone_id as u32);
        let track = clip.get_track_mut(bone_id as u32).unwrap();

        curve_add_keyframe(&mut track.rotation_z, 0.0, 0.0);
        curve_add_keyframe(&mut track.rotation_z, 1.0, 90.0);

        let pose = sample_role_clip(&clip, 0.5);

        let values = pose
            .rotations
            .get(&HumanoidRole::LeftUpperArm)
            .expect("LeftUpperArm should be in rotations (rotation_z has keyframes)");
        let rz = values[2];
        assert!(
            rz > 0.0 && rz < 90.0,
            "interpolated rotation_z at t=0.5 should be between 0 and 90, got {rz}",
        );
    }

    #[test]
    fn sample_role_clip_skips_unkeyed_tracks() {
        let bone_id = HumanoidRole::ALL
            .iter()
            .position(|r| *r == HumanoidRole::LeftUpperArm);
        let Some(bone_id) = bone_id else {
            panic!("LeftUpperArm not found in HumanoidRole::ALL");
        };

        let clip = make_clip_with_track(bone_id as u32);

        let pose = sample_role_clip(&clip, 0.0);

        assert!(
            !pose.rotations.contains_key(&HumanoidRole::LeftUpperArm),
            "unkeyed track should not appear in rotations",
        );
    }
}
