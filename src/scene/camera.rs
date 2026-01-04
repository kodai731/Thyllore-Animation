use cgmath::{Vector2, Vector3, Matrix3, InnerSpace};
use crate::math::*;

#[derive(Clone, Debug)]
pub struct Camera {
    position: Vector3<f32>,
    direction: Vector3<f32>,
    up: Vector3<f32>,
    initial_position: Vector3<f32>,
    near_plane: f32,
    far_plane: f32,
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

    pub fn position(&self) -> Vector3<f32> {
        self.position
    }

    pub fn direction(&self) -> Vector3<f32> {
        self.direction
    }

    pub fn up(&self) -> Vector3<f32> {
        self.up
    }

    pub fn initial_position(&self) -> Vector3<f32> {
        self.initial_position
    }

    pub fn near_plane(&self) -> f32 {
        self.near_plane
    }

    pub fn far_plane(&self) -> f32 {
        self.far_plane
    }

    pub fn set_near_plane(&mut self, near_plane: f32) {
        self.near_plane = near_plane;
    }

    pub fn set_far_plane(&mut self, far_plane: f32) {
        self.far_plane = far_plane;
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

    pub fn set_position(&mut self, position: Vector3<f32>) {
        self.position = position;
    }

    pub fn set_position_array(&mut self, position: [f32; 3]) {
        self.position = Vector3::new(position[0], position[1], position[2]);
    }

    pub fn set_direction(&mut self, direction: Vector3<f32>) {
        self.direction = direction.normalize();
    }

    pub fn set_direction_array(&mut self, direction: [f32; 3]) {
        self.direction = Vector3::new(direction[0], direction[1], direction[2]).normalize();
    }

    pub fn set_up(&mut self, up: Vector3<f32>) {
        self.up = up.normalize();
    }

    pub fn set_up_array(&mut self, up: [f32; 3]) {
        self.up = Vector3::new(up[0], up[1], up[2]).normalize();
    }

    pub fn set_initial_position(&mut self, position: Vector3<f32>) {
        self.initial_position = position;
    }

    pub fn reset(&mut self) {
        self.position = self.initial_position;
        self.direction = (Vector3::new(0.0, 0.0, 0.0) - self.position).normalize();
        self.up = Vector3::new(0.0, 1.0, 0.0);
        crate::log!("Camera::reset() - position: ({:.2}, {:.2}, {:.2}), direction: ({:.2}, {:.2}, {:.2})",
            self.position.x, self.position.y, self.position.z,
            self.direction.x, self.direction.y, self.direction.z);
    }

    pub fn reset_up(&mut self) {
        let horizon = self.up.cross(self.direction);
        self.up = Vector3::new(0.0, 1.0, 0.0);
        self.direction = horizon.cross(self.up).normalize();
    }

    pub fn look_at(&mut self, target: Vector3<f32>) {
        self.direction = (target - self.position).normalize();
    }

    pub fn move_to_look_at(&mut self, target: Vector3<f32>, offset: Vector3<f32>) {
        self.position = target + offset;
        self.direction = (target - self.position).normalize();
        self.up = Vector3::new(0.0, 1.0, 0.0);
    }

    pub fn rotate(&mut self, mouse_diff: Vector2<f32>) -> (Vector3<f32>, Vector3<f32>) {
        use crate::math::coordinate_system::world_y_axis;

        let world_y = world_y_axis();
        let camera_right = self.up.cross(self.direction).normalize();

        let mut rotate_x = Matrix3::identity();
        let mut rotate_y = Matrix3::identity();
        let theta_x = -mouse_diff.x * 0.005;
        let theta_y = mouse_diff.y * 0.005;

        unsafe {
            let _ = rodrigues(&mut rotate_x, theta_x.cos(), theta_x.sin(), &world_y);
            let _ = rodrigues(&mut rotate_y, theta_y.cos(), theta_y.sin(), &camera_right);
        }

        let rotate = rotate_y * rotate_x;
        self.up = rotate * self.up;
        self.direction = rotate * self.direction;

        self.direction = self.direction.normalize();
        let camera_right_new = self.up.cross(self.direction).normalize();
        self.up = self.direction.cross(camera_right_new).normalize();

        (self.direction, self.up)
    }

    pub fn pan(&mut self, mouse_diff: Vector2<f32>, speed: f32) {
        let camera_right = self.up.cross(self.direction).normalize();
        let translate_x = -camera_right * mouse_diff.x * speed;
        let translate_y = -self.up * mouse_diff.y * speed;
        self.position += translate_x + translate_y;
    }

    pub fn pan_with_base(&mut self, mouse_diff: Vector2<f32>, base_x: Vector3<f32>, base_y: Vector3<f32>, speed: f32) {
        let translate_x = -base_x * mouse_diff.x * speed;
        let translate_y = -base_y * mouse_diff.y * speed;
        self.position += translate_x + translate_y;
    }

    pub fn zoom(&mut self, mouse_wheel: f32, speed: f32) {
        let movement = self.direction * -mouse_wheel * speed;
        self.position += movement;
    }

    pub fn right(&self) -> Vector3<f32> {
        self.up.cross(self.direction).normalize()
    }
}
