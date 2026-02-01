use crate::ecs::resource::AnimationType;

#[derive(Clone, Debug)]
pub struct AnimationMeta {
    pub animation_type: AnimationType,
    pub node_animation_scale: f32,
}
