use std::collections::HashMap;
use std::path::PathBuf;

use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

use crate::animation::editable::SourceClipId;
use crate::animation::BoneId;

#[derive(Clone, Debug, Default)]
pub struct RecipeClipSources {
    pub by_clip: HashMap<SourceClipId, RecipeClipSource>,
}

#[derive(Clone, Debug)]
pub struct RecipeClipSource {
    pub path: PathBuf,
    pub pose_times: Vec<f32>,
    pub roles: Vec<(BoneId, HumanoidRole)>,
    pub detached: bool,
}
