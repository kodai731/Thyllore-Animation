use cgmath::{Quaternion, Vector3};

use crate::animation::spring_bone::DEFAULT_SIMULATION_STEP;
use crate::animation::BoneId;
use crate::ecs::component::SpringChainId;

#[derive(Clone, Debug)]
pub struct SpringJointState {
    pub prev_tail: Vector3<f32>,
    pub current_tail: Vector3<f32>,
    pub bone_length: f32,
    pub bone_axis: Vector3<f32>,
    pub initial_local_rotation: Quaternion<f32>,
    /// Distance from the rest tail to each collider axis of the chain, in collider order.
    pub rest_axis_distances: Vec<f32>,
    /// The joint's gravity direction in the parent's frame at initialization.
    pub gravity_in_parent_rest: Vector3<f32>,
}

impl Default for SpringJointState {
    fn default() -> Self {
        Self {
            prev_tail: Vector3::new(0.0, 0.0, 0.0),
            current_tail: Vector3::new(0.0, 0.0, 0.0),
            bone_length: 0.0,
            bone_axis: Vector3::new(0.0, 1.0, 0.0),
            initial_local_rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            rest_axis_distances: Vec::new(),
            gravity_in_parent_rest: Vector3::new(0.0, -1.0, 0.0),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpringChainState {
    pub chain_id: SpringChainId,
    pub joint_states: Vec<SpringJointState>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum SpringBoneMode {
    #[default]
    Realtime,
    Baked,
    BakedOverride,
}

#[derive(Clone, Debug)]
pub struct SpringBoneState {
    pub chain_states: Vec<SpringChainState>,
    pub initialized: bool,
    pub max_delta_time: f32,
    pub frame_count: u32,
    pub log_frames: u32,
    pub mode: SpringBoneMode,
    pub baked_clip_source_id: Option<u64>,
    pub baked_bone_ids: Vec<BoneId>,
    pub original_clip_source_id: Option<u64>,
}

impl Default for SpringBoneState {
    fn default() -> Self {
        Self {
            chain_states: Vec::new(),
            initialized: false,
            max_delta_time: DEFAULT_SIMULATION_STEP,
            frame_count: 0,
            log_frames: 10,
            mode: SpringBoneMode::Realtime,
            baked_clip_source_id: None,
            baked_bone_ids: Vec::new(),
            original_clip_source_id: None,
        }
    }
}
