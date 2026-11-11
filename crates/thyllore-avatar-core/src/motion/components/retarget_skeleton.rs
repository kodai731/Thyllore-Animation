use glam::{Quat, Vec3};

#[derive(Clone, Debug)]
pub struct RetargetBone {
    pub parent: Option<usize>,
    pub world_position: Vec3,
    pub world_rotation: Quat,
}

#[derive(Clone, Debug, Default)]
pub struct RetargetSkeleton {
    pub bones: Vec<RetargetBone>,
}
