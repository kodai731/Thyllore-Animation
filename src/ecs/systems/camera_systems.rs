use cgmath::{Deg, InnerSpace, Matrix3, SquareMatrix, Vector2, Vector3, Vector4};

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

pub fn camera_input_system(
    camera: &mut Camera,
    gui_data: &GUIData,
    grid_scale: f32,
    screen_size: [f32; 2],
) {
    camera_input_system_inner(
        camera,
        gui_data.is_right_clicked,
        gui_data.is_wheel_clicked,
        gui_data.mouse_wheel,
        gui_data.mouse_diff,
        grid_scale,
        screen_size,
    );
}

pub fn camera_input_system_inner(
    camera: &mut Camera,
    is_right_clicked: bool,
    is_wheel_clicked: bool,
    mouse_wheel: f32,
    mouse_diff: [f32; 2],
    grid_scale: f32,
    screen_size: [f32; 2],
) {
    let diff = Vector2::new(mouse_diff[0], mouse_diff[1]);

    if is_right_clicked && diff.magnitude() > 0.001 {
        camera_rotate(camera, diff);
    } else if is_wheel_clicked && diff.magnitude() > 0.001 {
        let screen = Vector2::new(screen_size[0], screen_size[1]);
        camera_pan_projection(camera, diff, screen, grid_scale);
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
    camera.direction = (rotate * camera.direction).normalize();

    let world_up = world_y_axis();
    let right_candidate = camera.direction.cross(world_up);
    if right_candidate.magnitude2() > 1e-6 {
        let right = right_candidate.normalize();
        camera.up = right.cross(camera.direction).normalize();
    } else {
        camera.up = rotate * camera.up;
        let right = camera.up.cross(camera.direction).normalize();
        camera.up = camera.direction.cross(right).normalize();
    }

    (camera.direction, camera.up)
}

pub fn camera_pan_projection(
    camera: &mut Camera,
    mouse_diff: Vector2<f32>,
    screen_size: Vector2<f32>,
    pivot_distance: f32,
) {
    let view_matrix = unsafe {
        crate::math::view(camera.position, camera.direction, camera.up)
    };
    let aspect = screen_size.x / screen_size.y;
    let proj_matrix = crate::math::perspective(
        Deg(45.0),
        aspect,
        camera.near_plane,
        camera.far_plane,
    );
    let vp = proj_matrix * view_matrix;
    let Some(vp_inv) = vp.invert() else {
        return;
    };

    let pivot = camera.position + camera.direction * pivot_distance;
    let pivot_clip = vp * Vector4::new(pivot.x, pivot.y, pivot.z, 1.0);
    let ndc_depth = pivot_clip.z / pivot_clip.w;

    let ndc_dx = -2.0 * mouse_diff.x / screen_size.x;
    let ndc_dy = 2.0 * mouse_diff.y / screen_size.y;

    let center_world = vp_inv * Vector4::new(0.0, 0.0, ndc_depth, 1.0);
    let shifted_world = vp_inv * Vector4::new(ndc_dx, ndc_dy, ndc_depth, 1.0);

    let center = Vector3::new(
        center_world.x / center_world.w,
        center_world.y / center_world.w,
        center_world.z / center_world.w,
    );
    let shifted = Vector3::new(
        shifted_world.x / shifted_world.w,
        shifted_world.y / shifted_world.w,
        shifted_world.z / shifted_world.w,
    );

    camera.position += shifted - center;
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
