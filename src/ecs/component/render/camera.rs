use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct CameraState {
    pub position: Vec3,
    pub direction: Vec3,
}
