use cgmath::Matrix4;

use crate::animation::SkeletonId;
use crate::ecs::resource::AnimationType;

pub struct AnimationEvalResult {
    pub updated_meshes: Vec<usize>,
    pub bone_transforms: Option<(SkeletonId, Vec<Matrix4<f32>>, AnimationType)>,
}

#[derive(Clone)]
pub(crate) struct ActiveInstanceInfo {
    pub(crate) source_id: crate::animation::editable::SourceClipId,
    pub(crate) asset_id: crate::asset::AssetId,
    pub(crate) instance_id: crate::animation::editable::ClipInstanceId,
    pub(crate) local_time: f32,
    pub(crate) weight: f32,
    pub(crate) blend_mode: crate::animation::editable::BlendMode,
    pub(crate) ease_out: crate::animation::editable::EaseType,
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
