use std::collections::BTreeMap;

use cgmath::{Quaternion, Vector3};

#[derive(Clone, Debug)]
pub struct BakedMotion {
    pub fps: u32,
    pub frame_times: Vec<f32>,
    pub bone_rotations: BTreeMap<usize, Vec<Quaternion<f32>>>,
    pub hips_offsets: Vec<Vector3<f32>>,
    pub morph_weights: BTreeMap<String, Vec<f32>>,
}
