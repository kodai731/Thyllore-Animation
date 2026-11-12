use std::collections::BTreeMap;

use glam::{Quat, Vec3};

#[derive(Clone, Debug)]
pub struct BakedMotion {
    pub fps: u32,
    pub frame_times: Vec<f32>,
    pub bone_rotations: BTreeMap<usize, Vec<Quat>>,
    pub hips_offsets: Vec<Vec3>,
    pub morph_weights: BTreeMap<String, Vec<f32>>,
}
