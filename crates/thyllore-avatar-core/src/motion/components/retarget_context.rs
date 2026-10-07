use std::collections::BTreeMap;

use cgmath::{Quaternion, Vector3};

use crate::humanoid::components::humanoid_frame::HumanoidFrame;
use crate::humanoid::components::mapping::HumanoidMapping;

use super::retarget_skeleton::RetargetSkeleton;

#[derive(Clone, Debug)]
pub struct RetargetContext {
    pub skeleton: RetargetSkeleton,
    pub mapping: HumanoidMapping,
    pub frame: HumanoidFrame,
    pub tpose_world: Vec<Quaternion<f32>>,
    pub hips_height: f32,
}

#[derive(Clone, Debug)]
pub struct RetargetedPose {
    pub local_rotations: BTreeMap<usize, Quaternion<f32>>,
    pub hips_offset: Vector3<f32>,
}
