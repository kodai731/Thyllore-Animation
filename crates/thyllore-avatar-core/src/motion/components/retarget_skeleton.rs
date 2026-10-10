use cgmath::{Quaternion, Vector3};

#[derive(Clone, Debug)]
pub struct RetargetBone {
    pub parent: Option<usize>,
    pub world_position: Vector3<f32>,
    pub world_rotation: Quaternion<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct RetargetSkeleton {
    pub bones: Vec<RetargetBone>,
}
