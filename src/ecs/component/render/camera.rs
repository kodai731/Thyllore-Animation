use cgmath::Vector3;

#[derive(Clone, Copy, Debug)]
pub struct CameraState {
    pub position: Vector3<f32>,
    pub direction: Vector3<f32>,
}
