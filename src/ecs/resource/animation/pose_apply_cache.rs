use std::collections::HashMap;

use glam::Mat4;

#[derive(Default)]
pub struct PoseApplyCache {
    pub skinned_cache: HashMap<usize, Vec<Mat4>>,
    pub node_cache: HashMap<usize, (Mat4, f32)>,
}
