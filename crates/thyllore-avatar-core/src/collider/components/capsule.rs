use cgmath::Vector3;

/// A vertical capsule standing on the widest cross-section of the skin the root bone drives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadCapsule {
    pub bottom: Vector3<f32>,
    pub top: Vector3<f32>,
    pub radius: f32,
}
