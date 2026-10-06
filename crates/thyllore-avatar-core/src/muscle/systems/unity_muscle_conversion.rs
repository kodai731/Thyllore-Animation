use std::collections::BTreeMap;

use thyllore_math_core::muscle::{degrees_to_muscle, muscle_to_degrees};

use crate::humanoid::components::role::HumanoidRole;
use crate::motion::components::sampled_pose::SampledPose;

use crate::muscle::components::unity_muscle_table::UnityMuscleTable;

pub fn to_unity_muscles(pose: &SampledPose, table: &UnityMuscleTable) -> Vec<f32> {
    table
        .muscles
        .iter()
        .map(|muscle| {
            let angle = pose
                .rotations
                .get(&muscle.role)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0])[muscle.axis];
            degrees_to_muscle(angle, muscle.sign, muscle.min, muscle.max, muscle.rest)
        })
        .collect()
}

pub fn from_unity_muscles(values: &[f32], table: &UnityMuscleTable) -> SampledPose {
    let mut angles: BTreeMap<HumanoidRole, [f32; 3]> = BTreeMap::new();

    for (i, muscle) in table.muscles.iter().enumerate() {
        if i >= values.len() {
            break;
        }
        let angle = muscle_to_degrees(values[i], muscle.sign, muscle.min, muscle.max, muscle.rest);
        let entry = angles.entry(muscle.role).or_insert([0.0, 0.0, 0.0]);
        entry[muscle.axis] = angle;
    }

    SampledPose {
        rotations: angles,
        hips_translation: [0.0, 0.0, 0.0],
        morph: BTreeMap::new(),
    }
}
