use crate::app::{App, AppData};
use rust_rendering::math::math::*;
use cgmath::Vector3;

impl App {
    pub unsafe fn reset_camera(&mut self) {
        self.data.camera_pos = self.data.initial_camera_pos;
        let camera_pos = vec3_from_array(self.data.camera_pos);
        let camera_direction = (vec3(0.0, 0.0, 0.0) - camera_pos).normalize();
        let camera_up = vec3(0.0, -1.0, 0.0);
        self.data.camera_direction = array3_from_vec(camera_direction);
        self.data.camera_up = array3_from_vec(camera_up);
    }

    pub unsafe fn reset_camera_up(&mut self) {
        let camera_pos = vec3_from_array(self.data.camera_pos);
        let mut camera_direction = vec3_from_array(self.data.camera_direction);
        let mut camera_up = vec3_from_array(self.data.camera_up);
        let horizon = Vector3::cross(camera_up, camera_direction);
        camera_up = vec3(0.0, -1.0, 0.0);
        camera_direction = Vector3::cross(horizon, camera_up);
        self.data.camera_up = array3_from_vec(camera_up);
        self.data.camera_direction = array3_from_vec(camera_direction);
    }
}
