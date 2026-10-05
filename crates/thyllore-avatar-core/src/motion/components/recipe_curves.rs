use std::collections::BTreeMap;

use crate::humanoid::components::role::HumanoidRole;
use crate::motion::components::pose_recipe::RecipeEase;

pub use super::sampled_pose::SampledPose;

#[derive(Clone, Debug)]
pub struct RecipeKey {
    pub time: f32,
    pub value: f32,
    pub ease: RecipeEase,
}

#[derive(Clone, Debug)]
pub struct RecipeCurves {
    pub rotations: BTreeMap<HumanoidRole, [Vec<RecipeKey>; 3]>,
    pub hips_translation: [Vec<RecipeKey>; 3],
    pub morph: BTreeMap<String, Vec<RecipeKey>>,
    pub duration_seconds: f32,
    pub fps: u32,
}
