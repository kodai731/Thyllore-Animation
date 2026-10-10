use std::collections::HashMap;

use thyllore_anim_core::BoneId;
use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;
use thyllore_avatar_core::motion::components::retarget_context::RetargetContext;

#[derive(Clone, Debug)]
pub struct HumanoidRig {
    pub confirmed: bool,
    pub mapping: HumanoidMapping,
    pub context: RetargetContext,
    pub track_bones: HashMap<String, BoneId>,
    pub track_names: HashMap<BoneId, String>,
}

#[derive(Clone, Debug, Default)]
pub struct HumanoidRigState {
    pub source_model_path: String,
    pub revision: u64,
    pub rig: Option<HumanoidRig>,
}
