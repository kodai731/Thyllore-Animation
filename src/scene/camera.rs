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

impl Camera {
    pub fn new(position: Vector3<f32>, target: Vector3<f32>) -> Self {
        let direction = (target - position).normalize();
        Self {
            position,
            direction,
            up: Vector3::new(0.0, 1.0, 0.0),
            initial_position: position,
            near_plane: 0.1,
            far_plane: 1000.0,
        }
    }

    pub fn position_array(&self) -> [f32; 3] {
        [self.position.x, self.position.y, self.position.z]
    }

    pub fn direction_array(&self) -> [f32; 3] {
        [self.direction.x, self.direction.y, self.direction.z]
    }

    pub fn up_array(&self) -> [f32; 3] {
        [self.up.x, self.up.y, self.up.z]
    }
}
