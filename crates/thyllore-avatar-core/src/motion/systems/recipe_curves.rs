use std::collections::BTreeMap;

use crate::humanoid::components::role::HumanoidRole;
use crate::motion::components::pose_recipe::{PoseRecipe, RecipeEase};
use crate::motion::components::recipe_curves::{RecipeCurves, RecipeKey, SampledPose};

pub fn build_recipe_curves(recipe: &PoseRecipe) -> RecipeCurves {
    let mut expanded_poses = recipe.poses.clone();

    if let Some(cycle) = &recipe.cycle {
        let interval = cycle.end - cycle.start;

        for pose in &recipe.poses {
            if pose.time >= cycle.start && pose.time < cycle.end {
                for k in 1..=cycle.count {
                    let mut repeated_pose = pose.clone();
                    repeated_pose.time = pose.time + k as f32 * interval;
                    expanded_poses.push(repeated_pose);
                }
            }
        }
    }

    expanded_poses.sort_by(|a, b| a.time.total_cmp(&b.time));

    let mut rotations: BTreeMap<HumanoidRole, [Vec<RecipeKey>; 3]> = BTreeMap::new();
    let mut hips_translation: [Vec<RecipeKey>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    let mut morph: BTreeMap<String, Vec<RecipeKey>> = BTreeMap::new();

    for pose in &expanded_poses {
        for (role, euler) in &pose.rotations {
            let curves = rotations
                .entry(*role)
                .or_insert_with(|| [Vec::new(), Vec::new(), Vec::new()]);
            for axis in 0..3 {
                curves[axis].push(RecipeKey {
                    time: pose.time,
                    value: euler[axis],
                    ease: pose.ease,
                });
            }
        }

        if let Some(translation) = &pose.hips_translation {
            for axis in 0..3 {
                hips_translation[axis].push(RecipeKey {
                    time: pose.time,
                    value: translation[axis],
                    ease: pose.ease,
                });
            }
        }

        for (name, weight) in &pose.morph {
            morph.entry(name.clone()).or_default().push(RecipeKey {
                time: pose.time,
                value: *weight,
                ease: pose.ease,
            });
        }
    }

    RecipeCurves {
        rotations,
        hips_translation,
        morph,
        duration_seconds: recipe.duration_seconds,
        fps: recipe.fps,
    }
}

pub fn sample_recipe_curve(keys: &[RecipeKey], time: f32) -> Option<f32> {
    let first = keys.first()?;
    let last = keys.last()?;

    if time <= first.time {
        return Some(first.value);
    }
    if time >= last.time {
        return Some(last.value);
    }

    let end_index = keys.partition_point(|key| key.time <= time);
    let start = &keys[end_index - 1];
    let end = &keys[end_index];
    let u = (time - start.time) / (end.time - start.time);
    Some(start.value + ease_time(u, end.ease) * (end.value - start.value))
}

fn ease_time(u: f32, ease: RecipeEase) -> f32 {
    match ease {
        RecipeEase::Linear => u,
        RecipeEase::EaseIn => u * u,
        RecipeEase::EaseOut => 1.0 - (1.0 - u) * (1.0 - u),
        RecipeEase::EaseInOut => 3.0 * u * u - 2.0 * u * u * u,
    }
}

pub fn sample_recipe_pose(curves: &RecipeCurves, time: f32) -> SampledPose {
    let mut rotations = BTreeMap::new();
    let mut hips_translation = [0.0f32; 3];
    let mut morph = BTreeMap::new();

    for (role, axis_curves) in &curves.rotations {
        let mut values = [0.0f32; 3];
        let mut has_any = false;
        for axis in 0..3 {
            if let Some(v) = sample_recipe_curve(&axis_curves[axis], time) {
                values[axis] = v;
                has_any = true;
            }
        }
        if has_any {
            rotations.insert(*role, values);
        }
    }

    for axis in 0..3 {
        if let Some(v) = sample_recipe_curve(&curves.hips_translation[axis], time) {
            hips_translation[axis] = v;
        }
    }

    for (name, keys) in &curves.morph {
        if let Some(v) = sample_recipe_curve(keys, time) {
            morph.insert(name.clone(), v);
        }
    }

    SampledPose {
        rotations,
        hips_translation,
        morph,
    }
}
