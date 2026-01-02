use crate::app::{App, AppData};
use rust_rendering::math::*;
use rust_rendering::logger::logger::*;
use cgmath::{Vector2, Vector3, Matrix3, InnerSpace};

impl App {
    pub unsafe fn reset_camera(&mut self) {
        self.data.camera_pos = self.data.initial_camera_pos;
        let camera_pos = vec3_from_array(self.data.camera_pos);
        let camera_direction = (vec3(0.0, 0.0, 0.0) - camera_pos).normalize();
        let camera_up = vec3(0.0, 1.0, 0.0);
        self.data.camera_direction = array3_from_vec(camera_direction);
        self.data.camera_up = array3_from_vec(camera_up);
    }

    pub unsafe fn reset_camera_up(&mut self) {
        let camera_pos = vec3_from_array(self.data.camera_pos);
        let mut camera_direction = vec3_from_array(self.data.camera_direction);
        let mut camera_up = vec3_from_array(self.data.camera_up);
        let horizon = Vector3::cross(camera_up, camera_direction);
        camera_up = vec3(0.0, 1.0, 0.0);
        camera_direction = Vector3::cross(horizon, camera_up);
        self.data.camera_up = array3_from_vec(camera_up);
        self.data.camera_direction = array3_from_vec(camera_direction);
    }

    pub unsafe fn move_camera_to_light(&mut self) {
        let light_pos = self.data.rt_debug_state.light_position;
        let offset_distance = 2.0;
        let offset = vec3(offset_distance, offset_distance, offset_distance);
        let camera_pos = light_pos + offset;
        let camera_direction = (light_pos - camera_pos).normalize();
        let camera_up = vec3(0.0, 1.0, 0.0);

        self.data.camera_pos = array3_from_vec(camera_pos);
        self.data.camera_direction = array3_from_vec(camera_direction);
        self.data.camera_up = array3_from_vec(camera_up);
    }

    pub unsafe fn rotate_camera(&mut self, mouse_diff: Vector2<f32>) -> (Vector3<f32>, Vector3<f32>) {
        let mut camera_pos = vec3_from_array(self.data.camera_pos);
        let mut camera_direction = vec3_from_array(self.data.camera_direction);
        let mut camera_up = vec3_from_array(self.data.camera_up);

        use rust_rendering::math::coordinate_system::world_y_axis;
        let world_y = world_y_axis();
        let camera_right = camera_up.cross(camera_direction).normalize();

        let mut rotate_x = Matrix3::identity();
        let mut rotate_y = Matrix3::identity();
        let theta_x = -mouse_diff.x * 0.005;
        let theta_y = mouse_diff.y * 0.005;

        let _ = rodrigues(
            &mut rotate_x,
            theta_x.cos(),
            theta_x.sin(),
            &world_y,
        );

        let _ = rodrigues(
            &mut rotate_y,
            theta_y.cos(),
            theta_y.sin(),
            &camera_right,
        );

        let rotate = rotate_y * rotate_x;
        camera_up = rotate * camera_up;
        camera_direction = rotate * camera_direction;

        camera_direction = camera_direction.normalize();
        let camera_right_new = camera_up.cross(camera_direction).normalize();
        camera_up = camera_direction.cross(camera_right_new).normalize();

        self.data.camera_direction = array3_from_vec(camera_direction);
        self.data.camera_up = array3_from_vec(camera_up);

        (camera_direction, camera_up)
    }

    pub unsafe fn pan_camera(&mut self, mouse_diff: Vector2<f32>, base_x: Vector3<f32>, base_y: Vector3<f32>) {
        static mut PAN_LOG_COUNTER: u32 = 0;

        let mut camera_pos = vec3_from_array(self.data.camera_pos);

        let pan_speed = self.data.grid_scale * 0.01;
        let translate_x_v = -base_x * mouse_diff.x * pan_speed;
        let translate_y_v = -base_y * mouse_diff.y * pan_speed;

        PAN_LOG_COUNTER += 1;
        if PAN_LOG_COUNTER % 30 == 0 {
            log!("=== Camera Pan (frame {}) ===", PAN_LOG_COUNTER);
            log!("  Mouse diff: ({:.3}, {:.3})", mouse_diff.x, mouse_diff.y);
            log!("  Before pan:");
            log!("    position: ({:.3}, {:.3}, {:.3})", camera_pos.x, camera_pos.y, camera_pos.z);
            log!("  Base vectors:");
            log!("    base_x (camera right): ({:.3}, {:.3}, {:.3})", base_x.x, base_x.y, base_x.z);
            log!("    base_y (camera up): ({:.3}, {:.3}, {:.3})", base_y.x, base_y.y, base_y.z);
            log!("  Translation vectors:");
            log!("    translate_x: ({:.3}, {:.3}, {:.3})", translate_x_v.x, translate_x_v.y, translate_x_v.z);
            log!("    translate_y: ({:.3}, {:.3}, {:.3})", translate_y_v.x, translate_y_v.y, translate_y_v.z);
        }

        camera_pos += translate_x_v + translate_y_v;

        if PAN_LOG_COUNTER % 30 == 0 {
            log!("  After pan:");
            log!("    position: ({:.3}, {:.3}, {:.3})", camera_pos.x, camera_pos.y, camera_pos.z);
        }

        self.data.camera_pos = array3_from_vec(camera_pos);
    }

    pub unsafe fn zoom_camera(&mut self, mouse_wheel: f32) {
        static mut ZOOM_LOG_COUNTER: u32 = 0;

        let mut camera_pos = vec3_from_array(self.data.camera_pos);
        let camera_direction = vec3_from_array(self.data.camera_direction);

        let zoom_speed = self.data.grid_scale * 0.5;
        let diff_view = camera_direction * -mouse_wheel * zoom_speed;

        ZOOM_LOG_COUNTER += 1;
        if ZOOM_LOG_COUNTER % 10 == 0 {
            log!("=== Camera Zoom (frame {}) ===", ZOOM_LOG_COUNTER);
            log!("  Mouse wheel: {:.3}", mouse_wheel);
            log!("  Grid scale: {:.3}", self.data.grid_scale);
            log!("  Zoom speed: {:.3}", zoom_speed);
            log!("  Before zoom:");
            log!("    position: ({:.3}, {:.3}, {:.3})", camera_pos.x, camera_pos.y, camera_pos.z);
            log!("    camera direction: ({:.3}, {:.3}, {:.3})", camera_direction.x, camera_direction.y, camera_direction.z);
            log!("  Movement vector (diff_view): ({:.3}, {:.3}, {:.3})", diff_view.x, diff_view.y, diff_view.z);
        }

        camera_pos += diff_view;

        if ZOOM_LOG_COUNTER % 10 == 0 {
            log!("  After zoom:");
            log!("    position: ({:.3}, {:.3}, {:.3})", camera_pos.x, camera_pos.y, camera_pos.z);
        }

        self.data.camera_pos = array3_from_vec(camera_pos);
    }
}
