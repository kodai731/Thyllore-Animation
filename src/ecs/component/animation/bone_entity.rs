use crate::animation::{BoneId, SkeletonId};
use crate::ecs::world::Entity;

#[derive(Clone, Debug)]
pub struct BoneEntity {
    pub skeleton_id: SkeletonId,
    pub bone_id: BoneId,
}

#[derive(Clone, Debug)]
pub struct SkeletonBones {
    pub skeleton_id: SkeletonId,
    pub entities: Vec<Entity>,
}

impl SkeletonBones {
    pub fn entity(&self, bone: BoneId) -> Option<Entity> {
        self.entities.get(bone as usize).copied()
    }
}
