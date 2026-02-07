use cgmath::{Deg, InnerSpace, Rad, Vector2, Vector3};

use crate::app::GUIData;
use crate::ecs::resource::Camera;

pub fn create_camera(
    position: Vector3<f32>,
    target: Vector3<f32>,
) -> Camera {
    let diff = position - target;
    let distance = diff.magnitude();
    let yaw = diff.x.atan2(diff.z);
    let pitch = (diff.y / distance).asin();

    Camera {
        pivot: target,
        yaw,
        pitch,
        distance,
        fov_y: Deg(45.0),
        near_plane: 0.1,
        initial_pivot: target,
        initial_yaw: yaw,
        initial_pitch: pitch,
        initial_distance: distance,
    }
}

fn compute_camera_backward(camera: &Camera) -> Vector3<f32> {
    Vector3::new(
        camera.pitch.cos() * camera.yaw.sin(),
        camera.pitch.sin(),
        camera.pitch.cos() * camera.yaw.cos(),
    )
}

pub fn compute_camera_position(camera: &Camera) -> Vector3<f32> {
    camera.pivot + compute_camera_backward(camera) * camera.distance
}

pub fn compute_camera_direction(camera: &Camera) -> Vector3<f32> {
    -compute_camera_backward(camera)
}

pub fn compute_camera_right(camera: &Camera) -> Vector3<f32> {
    let world_up = Vector3::new(0.0, 1.0, 0.0);
    let direction = compute_camera_direction(camera);
    direction.cross(world_up).normalize()
}

pub fn compute_camera_up(camera: &Camera) -> Vector3<f32> {
    let right = compute_camera_right(camera);
    let backward = compute_camera_backward(camera);
    right.cross(-backward).normalize()
}

pub fn camera_input_system(
    camera: &mut Camera,
    gui_data: &GUIData,
    screen_size: [f32; 2],
) {
    camera_input_system_inner(
        camera,
        gui_data.is_right_clicked,
        gui_data.is_wheel_clicked,
        gui_data.mouse_wheel,
        gui_data.mouse_diff,
        screen_size,
    );
}

pub fn camera_input_system_inner(
    camera: &mut Camera,
    is_right_clicked: bool,
    is_wheel_clicked: bool,
    mouse_wheel: f32,
    mouse_diff: [f32; 2],
    screen_size: [f32; 2],
) {
    let diff = Vector2::new(mouse_diff[0], mouse_diff[1]);

    if is_right_clicked && diff.magnitude() > 0.001 {
        camera_orbit(camera, diff);
    } else if is_wheel_clicked && diff.magnitude() > 0.001 {
        let screen = Vector2::new(screen_size[0], screen_size[1]);
        camera_pan(camera, diff, screen);
    }

    if mouse_wheel != 0.0 {
        camera_zoom(camera, mouse_wheel);
    }
}

pub fn camera_orbit(camera: &mut Camera, mouse_diff: Vector2<f32>) {
    let sensitivity = 0.005;
    camera.yaw -= mouse_diff.x * sensitivity;
    camera.pitch += mouse_diff.y * sensitivity;

    let max_pitch = std::f32::consts::FRAC_PI_2 - 0.001;
    camera.pitch = camera.pitch.clamp(-max_pitch, max_pitch);
}

pub fn camera_pan(
    camera: &mut Camera,
    mouse_diff: Vector2<f32>,
    screen_size: Vector2<f32>,
) {
    let right = compute_camera_right(camera);
    let up = compute_camera_up(camera);

    let fov_rad: Rad<f32> = camera.fov_y.into();
    let pan_speed =
        camera.distance * 2.0 * (fov_rad.0 / 2.0).tan() / screen_size.y;

    camera.pivot += right * (-mouse_diff.x * pan_speed);
    camera.pivot += up * (-mouse_diff.y * pan_speed);
}

pub fn camera_zoom(camera: &mut Camera, mouse_wheel: f32) {
    let zoom_factor = (-mouse_wheel * 0.1).exp();
    camera.distance *= zoom_factor;

    let min_distance = camera.near_plane * 2.0;
    camera.distance = camera.distance.max(min_distance);
}

pub fn camera_reset(camera: &mut Camera) {
    camera.pivot = camera.initial_pivot;
    camera.yaw = camera.initial_yaw;
    camera.pitch = camera.initial_pitch;
    camera.distance = camera.initial_distance;

    let position = compute_camera_position(camera);
    crate::log!(
        "camera_reset - position: ({:.2}, {:.2}, {:.2}), \
         pivot: ({:.2}, {:.2}, {:.2})",
        position.x,
        position.y,
        position.z,
        camera.pivot.x,
        camera.pivot.y,
        camera.pivot.z
    );
}

pub fn camera_look_at(camera: &mut Camera, target: Vector3<f32>) {
    camera.pivot = target;
}

pub fn camera_move_to_look_at(
    camera: &mut Camera,
    target: Vector3<f32>,
    offset: Vector3<f32>,
) {
    let new_position = target + offset;
    let diff = new_position - target;
    let distance = diff.magnitude();
    let yaw = diff.x.atan2(diff.z);
    let pitch = (diff.y / distance).asin();

    camera.pivot = target;
    camera.yaw = yaw;
    camera.pitch = pitch;
    camera.distance = distance;
}
