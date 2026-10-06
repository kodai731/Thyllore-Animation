use std::collections::BTreeMap;

use crate::humanoid::components::role::HumanoidRole;
use crate::motion::components::sampled_pose::SampledPose;

use crate::muscle::components::unity_muscle_table::{MuscleSlope, UnityMuscle, UnityMuscleTable};

const INVERSE_ITERATIONS: usize = 32;
const MIN_SLOPE: f32 = 1e-8;

pub fn to_unity_muscles(pose: &SampledPose, table: &UnityMuscleTable) -> Vec<f32> {
    table
        .muscles
        .iter()
        .map(|muscle| compute_muscle_value(muscle, pose))
        .collect()
}

fn compute_muscle_value(muscle: &UnityMuscle, pose: &SampledPose) -> f32 {
    let mut value = muscle.rest;
    for slope in &muscle.slopes {
        let angle = get_angle(pose, slope.role, slope.axis);
        let contribution = if angle >= 0.0 {
            slope.positive * angle
        } else {
            slope.negative * angle
        };
        value += contribution;
    }
    value
}

fn get_angle(pose: &SampledPose, role: HumanoidRole, axis: usize) -> f32 {
    pose.rotations
        .get(&role)
        .copied()
        .unwrap_or([0.0, 0.0, 0.0])[axis]
}

pub fn from_unity_muscles(values: &[f32], table: &UnityMuscleTable) -> SampledPose {
    let primary_slopes = build_primary_slopes(table);
    let mut angles: BTreeMap<HumanoidRole, [f32; 3]> = BTreeMap::new();

    for _ in 0..INVERSE_ITERATIONS {
        let current_values = to_unity_muscles(
            &SampledPose {
                rotations: angles.clone(),
                hips_translation: [0.0, 0.0, 0.0],
                morph: BTreeMap::new(),
            },
            table,
        );

        let mut residuals: BTreeMap<HumanoidRole, [f32; 3]> = BTreeMap::new();
        for (i, _muscle) in table.muscles.iter().enumerate() {
            if i >= values.len() {
                break;
            }
            let residual = values[i] - current_values[i];
            let Some(Some(primary)) = primary_slopes.get(i).copied() else {
                continue;
            };
            let role = primary.role;
            let axis = primary.axis;
            let entry = residuals.entry(role).or_insert([0.0, 0.0, 0.0]);
            let current_angle = angles.get(&role).copied().unwrap_or([0.0, 0.0, 0.0])[axis];
            let slope = if current_angle >= 0.0 {
                primary.positive
            } else {
                primary.negative
            };
            if slope.abs() > MIN_SLOPE {
                entry[axis] += residual / slope;
            }
        }

        for (role, correction) in residuals {
            let entry = angles.entry(role).or_insert([0.0, 0.0, 0.0]);
            for axis in 0..3 {
                entry[axis] += correction[axis];
            }
        }
    }

    SampledPose {
        rotations: angles,
        hips_translation: [0.0, 0.0, 0.0],
        morph: BTreeMap::new(),
    }
}

fn build_primary_slopes(table: &UnityMuscleTable) -> Vec<Option<&MuscleSlope>> {
    table
        .muscles
        .iter()
        .map(|muscle| {
            let own_slopes = muscle.slopes.iter().filter(|s| s.role == muscle.role);
            own_slopes.max_by(|a, b| {
                let mag_a = a.positive.abs().max(a.negative.abs());
                let mag_b = b.positive.abs().max(b.negative.abs());
                mag_a.partial_cmp(&mag_b).unwrap()
            })
        })
        .collect::<Vec<_>>()
}
