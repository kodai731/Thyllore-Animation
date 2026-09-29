use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::humanoid::components::role::HumanoidRole;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoseRecipe {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub prompt: String,
    pub duration_seconds: f32,
    pub fps: u32,
    #[serde(default, rename = "loop")]
    pub is_loop: bool,
    pub poses: Vec<RecipePose>,
    #[serde(default)]
    pub cycle: Option<RecipeCycle>,
    #[serde(default)]
    pub hand_presets: BTreeMap<HumanoidRole, HandPreset>,
    #[serde(default)]
    pub copilot: RecipeCopilot,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecipePose {
    pub time: f32,
    #[serde(default)]
    pub ease: RecipeEase,
    #[serde(default)]
    pub rotations: BTreeMap<HumanoidRole, [f32; 3]>,
    #[serde(default)]
    pub hips_translation: Option<[f32; 3]>,
    #[serde(default)]
    pub morph: BTreeMap<String, f32>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeEase {
    Linear,
    EaseIn,
    EaseOut,
    #[default]
    EaseInOut,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeCycle {
    pub start: f32,
    pub end: f32,
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandPreset {
    Open,
    Fist,
    Point,
    Peace,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeCopilot {
    #[serde(default)]
    pub extend: Vec<CopilotExtend>,
    #[serde(default)]
    pub ease: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CopilotExtend {
    pub curve: String,
    pub horizon_frames: u32,
}
