use cgmath::{InnerSpace, Matrix3, SquareMatrix, Vector2, Vector3};

use crate::app::GUIData;
use crate::math::{coordinate_system::world_y_axis, rodrigues};
use crate::scene::camera::Camera;

pub fn create_camera(position: Vector3<f32>, target: Vector3<f32>) -> Camera {
    let direction = (target - position).normalize();
    Camera {
        position,
        direction,
        up: Vector3::new(0.0, 1.0, 0.0),
        initial_position: position,
        near_plane: 0.1,
        far_plane: 1000.0,
    }
}

pub fn camera_input_system(camera: &mut Camera, gui_data: &GUIData, grid_scale: f32) {
    camera_input_system_inner(
        camera,
        gui_data.is_left_clicked,
        gui_data.is_wheel_clicked,
        gui_data.mouse_wheel,
        gui_data.mouse_diff,
        grid_scale,
    );
}

pub fn camera_input_system_inner(
    camera: &mut Camera,
    is_left_clicked: bool,
    is_wheel_clicked: bool,
    mouse_wheel: f32,
    mouse_diff: [f32; 2],
    grid_scale: f32,
) {
    let diff = Vector2::new(mouse_diff[0], mouse_diff[1]);

    if is_left_clicked && diff.magnitude() > 0.001 {
        camera_rotate(camera, diff);
    } else if is_wheel_clicked && diff.magnitude() > 0.001 {
        let base_x = camera_right(camera);
        let base_y = camera.up;
        let pan_speed = grid_scale * 0.01;
        camera_pan_with_base(camera, diff, base_x, base_y, pan_speed);
    }

    if mouse_wheel != 0.0 {
        let zoom_speed = grid_scale * 0.5;
        camera_zoom(camera, mouse_wheel, zoom_speed);
    }
}

pub fn camera_rotate(
    camera: &mut Camera,
    mouse_diff: Vector2<f32>,
) -> (Vector3<f32>, Vector3<f32>) {
    let world_y = world_y_axis();
    let cam_right = camera.up.cross(camera.direction).normalize();

    let mut rotate_x = Matrix3::identity();
    let mut rotate_y = Matrix3::identity();
    let theta_x = -mouse_diff.x * 0.005;
    let theta_y = mouse_diff.y * 0.005;

    unsafe {
        let _ = rodrigues(&mut rotate_x, theta_x.cos(), theta_x.sin(), &world_y);
        let _ = rodrigues(&mut rotate_y, theta_y.cos(), theta_y.sin(), &cam_right);
    }

    let rotate = rotate_y * rotate_x;
    camera.up = rotate * camera.up;
    camera.direction = rotate * camera.direction;

    camera.direction = camera.direction.normalize();
    let camera_right_new = camera.up.cross(camera.direction).normalize();
    camera.up = camera.direction.cross(camera_right_new).normalize();

    (camera.direction, camera.up)
}

pub fn camera_pan(camera: &mut Camera, mouse_diff: Vector2<f32>, speed: f32) {
    let cam_right = camera.up.cross(camera.direction).normalize();
    let translate_x = -cam_right * mouse_diff.x * speed;
    let translate_y = -camera.up * mouse_diff.y * speed;
    camera.position += translate_x + translate_y;
}

pub fn camera_pan_with_base(
    camera: &mut Camera,
    mouse_diff: Vector2<f32>,
    base_x: Vector3<f32>,
    base_y: Vector3<f32>,
    speed: f32,
) {
    let translate_x = base_x * mouse_diff.x * speed;
    let translate_y = base_y * mouse_diff.y * speed;
    camera.position += translate_x + translate_y;
}

pub fn camera_zoom(camera: &mut Camera, mouse_wheel: f32, speed: f32) {
    let movement = camera.direction * mouse_wheel * speed;
    camera.position += movement;
}

pub fn camera_reset(camera: &mut Camera) {
    camera.position = camera.initial_position;
    camera.direction = (Vector3::new(0.0, 0.0, 0.0) - camera.position).normalize();
    camera.up = Vector3::new(0.0, 1.0, 0.0);
    crate::log!(
        "camera_reset - position: ({:.2}, {:.2}, {:.2}), direction: ({:.2}, {:.2}, {:.2})",
        camera.position.x,
        camera.position.y,
        camera.position.z,
        camera.direction.x,
        camera.direction.y,
        camera.direction.z
    );
}

pub fn camera_reset_up(camera: &mut Camera) {
    let horizon = camera.up.cross(camera.direction);
    camera.up = Vector3::new(0.0, 1.0, 0.0);
    camera.direction = horizon.cross(camera.up).normalize();
}

pub fn camera_look_at(camera: &mut Camera, target: Vector3<f32>) {
    camera.direction = (target - camera.position).normalize();
}

pub fn camera_move_to_look_at(camera: &mut Camera, target: Vector3<f32>, offset: Vector3<f32>) {
    camera.position = target + offset;
    camera.direction = (target - camera.position).normalize();
    camera.up = Vector3::new(0.0, 1.0, 0.0);
}

pub fn camera_right(camera: &Camera) -> Vector3<f32> {
    camera.up.cross(camera.direction).normalize()
}
