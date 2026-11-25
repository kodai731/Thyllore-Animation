use glam::Vec3;

/// A vertical capsule standing on the widest cross-section of the skin the root bone drives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadCapsule {
    pub bottom: Vec3,
    pub top: Vec3,
    pub radius: f32,
}
