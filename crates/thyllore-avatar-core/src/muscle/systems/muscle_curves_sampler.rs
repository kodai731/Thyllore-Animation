use thyllore_anim_core::editable::components::clip::EditableAnimationClip;

use crate::humanoid::components::mapping::HumanoidMapping;
use crate::motion::systems::humanoid_pose_sampler::sample_humanoid_pose;
use crate::muscle::components::muscle_curves::MuscleCurves;
use crate::muscle::components::unity_muscle_table::UnityMuscleTable;
use crate::muscle::systems::unity_muscle_conversion::to_unity_muscles;

pub fn muscle_curves_from_clip(
    clip: &EditableAnimationClip,
    mapping: &HumanoidMapping,
    sample_rate: u32,
    table: &UnityMuscleTable,
) -> MuscleCurves {
    let duration = clip.duration;
    let num_frames = (duration * sample_rate as f32).ceil() as usize + 1;

    let mut times = Vec::with_capacity(num_frames);
    let mut frames = Vec::with_capacity(num_frames);
    for i in 0..num_frames {
        let time = if i == num_frames - 1 {
            duration
        } else {
            (i as f32) / sample_rate as f32
        };
        times.push(time);
        let pose = sample_humanoid_pose(clip, mapping, time);
        let values = to_unity_muscles(&pose, table);
        frames.push(values);
    }

    MuscleCurves {
        sample_rate,
        duration,
        times,
        frames,
    }
}
