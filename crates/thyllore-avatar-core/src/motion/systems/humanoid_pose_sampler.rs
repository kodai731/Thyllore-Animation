use std::collections::BTreeMap;

use thyllore_anim_core::editable::components::clip::EditableAnimationClip;
use thyllore_anim_core::editable::systems::curve_ops::curve_sample;
use thyllore_anim_core::editable::systems::morph_sample::sample_morph_tracks;

use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::role::HumanoidRole;
use crate::motion::components::sampled_pose::SampledPose;

pub fn sample_humanoid_pose(
    clip: &EditableAnimationClip,
    mapping: &HumanoidMapping,
    time: f32,
) -> SampledPose {
    let mut rotations: BTreeMap<HumanoidRole, [f32; 3]> = BTreeMap::new();
    let mut hips_translation = [0.0f32; 3];

    for track in clip.tracks.values() {
        let bone_id = track.bone_id as usize;

        let Some(role) = mapping
            .by_role
            .iter()
            .find(|(_, &idx)| idx == bone_id)
            .map(|(r, _)| *r)
        else {
            continue;
        };

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
