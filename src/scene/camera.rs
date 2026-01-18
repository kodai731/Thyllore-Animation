use cgmath::{InnerSpace, Vector3};

#[derive(Clone, Debug)]
pub struct Camera {
    pub position: Vector3<f32>,
    pub direction: Vector3<f32>,
    pub up: Vector3<f32>,
    pub initial_position: Vector3<f32>,
    pub near_plane: f32,
    pub far_plane: f32,
}

impl Default for Camera {
    fn default() -> Self {
        let initial_pos = Vector3::new(5.0, 5.0, 5.0);
        let direction = (Vector3::new(0.0, 0.0, 0.0) - initial_pos).normalize();
        Self {
            position: initial_pos,
            direction,
            up: Vector3::new(0.0, 1.0, 0.0),
            initial_position: initial_pos,
            near_plane: 0.1,
            far_plane: 1000.0,
        }
    }
}

