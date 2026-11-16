use thyllore_anim_core::spring_bone::SpringSimulation;
use thyllore_anim_core::BoneId;

#[derive(Clone, Debug, PartialEq, Default)]
pub enum SpringBoneMode {
    #[default]
    Realtime,
    Baked,
    BakedOverride,
}

#[derive(Clone, Debug, Default)]
pub struct SpringBoneState {
    pub simulation: SpringSimulation,
    pub initialized: bool,
    pub mode: SpringBoneMode,
    pub baked_clip_source_id: Option<u64>,
    pub baked_bone_ids: Vec<BoneId>,
    pub original_clip_source_id: Option<u64>,
}

crate::startup_resource!(SpringBoneState, Editor);
