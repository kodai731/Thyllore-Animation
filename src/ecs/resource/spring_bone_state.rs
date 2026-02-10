use cgmath::{Quaternion, Vector3};

use crate::ecs::component::SpringChainId;

#[derive(Clone, Debug)]
pub struct SpringJointState {
    pub prev_tail: Vector3<f32>,
    pub current_tail: Vector3<f32>,
    pub bone_length: f32,
    pub bone_axis: Vector3<f32>,
    pub initial_local_rotation: Quaternion<f32>,
}

impl Default for SpringJointState {
    fn default() -> Self {
        Self {
            prev_tail: Vector3::new(0.0, 0.0, 0.0),
            current_tail: Vector3::new(0.0, 0.0, 0.0),
            bone_length: 0.0,
            bone_axis: Vector3::new(0.0, 1.0, 0.0),
            initial_local_rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpringChainState {
    pub chain_id: SpringChainId,
    pub joint_states: Vec<SpringJointState>,
}

#[derive(Clone, Debug)]
pub struct SpringBoneState {
    pub chain_states: Vec<SpringChainState>,
    pub initialized: bool,
    pub max_delta_time: f32,
    pub frame_count: u32,
    pub log_frames: u32,
}

impl Default for SpringBoneState {
    fn default() -> Self {
        Self {
            chain_states: Vec::new(),
            initialized: false,
            max_delta_time: 1.0 / 30.0,
            frame_count: 0,
            log_frames: 10,
        }
    }
}
