use glam::Mat4;

use crate::ecs::resource::AnimationType;
use thyllore_anim_core::SkeletonId;

pub struct AnimationEvalResult {
    pub updated_meshes: Vec<usize>,
    pub bone_transforms: Option<(SkeletonId, Vec<Mat4>, AnimationType)>,
}

#[derive(Clone)]
pub(crate) struct ActiveInstanceInfo {
    pub(crate) asset_id: crate::asset::AssetId,
    pub(crate) local_time: f32,
    pub(crate) weight: f32,
    pub(crate) blend_mode: thyllore_anim_core::editable::BlendMode,
    pub(crate) ease_out: thyllore_anim_core::editable::EaseType,
    pub(crate) start_time: f32,
    pub(crate) end_time: f32,
}

pub(crate) struct AnimatedEntityInfo {
    pub(crate) entity: crate::ecs::world::Entity,
    pub(crate) active_instances: Vec<ActiveInstanceInfo>,
    pub(crate) skeleton_id: SkeletonId,
    pub(crate) mesh_idx: usize,
    pub(crate) animation_type: AnimationType,
    pub(crate) node_animation_scale: f32,
    pub(crate) looping: bool,
}
