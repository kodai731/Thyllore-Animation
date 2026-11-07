use std::collections::HashMap;

use glam::{UVec4, Vec4};
use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;

use crate::animation::editable::{BoneTrack, SourceClipId};
use crate::animation::{BoneId, Skeleton};
use crate::ecs::component::{ConstraintSet, SpringBoneSetup};
use crate::ecs::world::Entity;

/// Skin weights of one graphics mesh, identified by its index in the graphics resources.
#[derive(Clone, Debug)]
pub struct SkinSnapshot {
    pub mesh_index: usize,
    pub bone_indices: Vec<UVec4>,
    pub bone_weights: Vec<Vec4>,
}

/// Everything a skeleton edit can change, taken before and after the edit for undo / redo.
#[derive(Clone, Debug)]
pub struct SkeletonSnapshot {
    pub skeleton: Skeleton,
    pub skins: Vec<SkinSnapshot>,
    pub spring_setups: Vec<(Entity, SpringBoneSetup)>,
    pub constraint_sets: Vec<(Entity, ConstraintSet)>,
    pub clip_tracks: Vec<(SourceClipId, HashMap<BoneId, BoneTrack>)>,
    pub humanoid_mapping: HumanoidMapping,
}
