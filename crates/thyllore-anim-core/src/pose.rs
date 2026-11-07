use glam::{Quat, Vec3};

use crate::SkeletonId;

#[derive(Clone, Debug)]
pub struct BoneLocalPose {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for BoneLocalPose {
    fn default() -> Self {
        Self {
            translation: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quat::from_xyzw(0.0, 0.0, 0.0, 1.0),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SkeletonPose {
    pub skeleton_id: SkeletonId,
    pub bone_poses: Vec<BoneLocalPose>,
}
