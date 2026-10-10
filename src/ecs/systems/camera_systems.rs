use cgmath::{Deg, InnerSpace, Rad, Vector2, Vector3};
use thyllore_math_core::BoundingSphere;

use crate::ecs::resource::{Camera, CameraFlyInput, CameraPose, CameraTransition};

pub const CAMERA_FRAMING_MARGIN: f32 = 1.1;
pub const CAMERA_FRAMING_SECONDS: f32 = 0.25;
pub const FLY_SPEED_WHEEL_FACTOR: f32 = 1.08;
pub const FLY_SPEED_INDICATOR_SECONDS: f32 = 1.5;
const FLY_SPEED_SCALE_RANGE: (f32, f32) = (0.05, 50.0);

pub fn create_camera(position: Vector3<f32>, target: Vector3<f32>) -> Camera {
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
        transition: None,
        fly_speed_scale: Camera::DEFAULT_FLY_SPEED_SCALE,
        fly_speed_indicator_seconds: 0.0,
    }
}

pub fn compute_camera_backward(camera: &Camera) -> Vector3<f32> {
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

pub fn camera_input_system_inner(
    camera: &mut Camera,
    is_right_clicked: bool,
    is_wheel_clicked: bool,
    is_alt_held: bool,
    fly: &CameraFlyInput,
    mouse_wheel: f32,
    mouse_diff: [f32; 2],
    mouse_pos: [f32; 2],
    screen_size: [f32; 2],
) {
    let diff = Vector2::new(mouse_diff[0], mouse_diff[1]);
    let any_input = diff.magnitude() > 0.001
        || mouse_wheel != 0.0
        || (is_right_clicked && fly_direction_is_active(fly));
    if any_input {
        camera.transition = None;
    }

    if is_right_clicked && mouse_wheel != 0.0 {
        adjust_fly_speed(camera, mouse_wheel);
        return;
    }

    if is_right_clicked && is_alt_held {
        if diff.magnitude() > 0.001 {
            camera_orbit(camera, diff);
        }
    } else if is_right_clicked {
        if diff.magnitude() > 0.001 {
            camera_look(camera, diff);
        }
        camera_fly_move(camera, fly);
    } else if is_wheel_clicked && diff.magnitude() > 0.001 {
        let screen = Vector2::new(screen_size[0], screen_size[1]);
        camera_pan(camera, diff, screen);
    }

    if mouse_wheel != 0.0 {
        let pos = Vector2::new(mouse_pos[0], mouse_pos[1]);
        let screen = Vector2::new(screen_size[0], screen_size[1]);
        camera_zoom(camera, mouse_wheel, pos, screen);
    }
}

fn apply_yaw_pitch(camera: &mut Camera, mouse_diff: Vector2<f32>) {
    let sensitivity = 0.005;
    camera.yaw -= mouse_diff.x * sensitivity;
    camera.pitch += mouse_diff.y * sensitivity;

    let max_pitch = std::f32::consts::FRAC_PI_2 - 0.001;
    camera.pitch = camera.pitch.clamp(-max_pitch, max_pitch);
}

pub fn camera_orbit(camera: &mut Camera, mouse_diff: Vector2<f32>) {
    apply_yaw_pitch(camera, mouse_diff);
}

pub fn camera_look(camera: &mut Camera, mouse_diff: Vector2<f32>) {
    let position = compute_camera_position(camera);
    apply_yaw_pitch(camera, mouse_diff);
    camera.pivot = position - compute_camera_backward(camera) * camera.distance;
}

fn fly_direction_is_active(fly: &CameraFlyInput) -> bool {
    fly.forward != 0.0 || fly.right != 0.0 || fly.up != 0.0
}

pub fn camera_fly_move(camera: &mut Camera, fly: &CameraFlyInput) {
    let movement = compute_camera_direction(camera) * fly.forward
        + compute_camera_right(camera) * fly.right
        + compute_camera_up(camera) * fly.up;
    if movement.magnitude() < 1e-6 || fly.delta_seconds <= 0.0 {
        return;
    }

    let speed = camera_fly_speed(camera) * fly.speed_modifier.factor();
    camera.pivot += movement.normalize() * speed * fly.delta_seconds;
}

pub fn camera_fly_speed(camera: &Camera) -> f32 {
    camera.distance.max(0.5) * camera.fly_speed_scale
}

pub fn adjust_fly_speed(camera: &mut Camera, mouse_wheel: f32) {
    let factor = FLY_SPEED_WHEEL_FACTOR.powf(mouse_wheel);
    camera.fly_speed_scale =
        (camera.fly_speed_scale * factor).clamp(FLY_SPEED_SCALE_RANGE.0, FLY_SPEED_SCALE_RANGE.1);
    camera.fly_speed_indicator_seconds = FLY_SPEED_INDICATOR_SECONDS;
}

/// Unity `GetPerspectiveCameraDistance`: the sphere fills the narrower of the two view angles.
pub fn camera_pose_framing(camera: &Camera, bounds: BoundingSphere, aspect: f32) -> CameraPose {
    let radius = bounds.radius.max(camera.near_plane * 4.0) * CAMERA_FRAMING_MARGIN;
    let fov_y: Rad<f32> = camera.fov_y.into();
    let half_fov_y = fov_y.0 * 0.5;
    let half_fov_x = (half_fov_y.tan() * aspect.max(1e-3)).atan();
    let half_fov = half_fov_y.min(half_fov_x);

    CameraPose {
        pivot: bounds.center,
        yaw: camera.yaw,
        pitch: camera.pitch,
        distance: radius / half_fov.sin(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMotion {
    Eased,
    Immediate,
}

pub fn camera_move_to_pose(camera: &mut Camera, target: CameraPose, motion: CameraMotion) {
    match motion {
        CameraMotion::Immediate => {
            camera.transition = None;
            camera.set_pose(target);
        }
        CameraMotion::Eased => {
            camera.transition = Some(CameraTransition {
                from: camera.pose(),
                to: target,
                elapsed_seconds: 0.0,
                duration_seconds: CAMERA_FRAMING_SECONDS,
            });
        }
    }
}

pub fn advance_camera_transition(camera: &mut Camera, delta_seconds: f32) {
    let Some(mut transition) = camera.transition else {
        return;
    };
    transition.elapsed_seconds += delta_seconds.max(0.0);
    let progress = (transition.elapsed_seconds / transition.duration_seconds).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress).powi(3);

    camera.set_pose(lerp_camera_pose(transition.from, transition.to, eased));
    camera.transition = if progress >= 1.0 {
        None
    } else {
        Some(transition)
    };
}

fn lerp_camera_pose(from: CameraPose, to: CameraPose, t: f32) -> CameraPose {
    CameraPose {
        pivot: from.pivot + (to.pivot - from.pivot) * t,
        yaw: from.yaw + (to.yaw - from.yaw) * t,
        pitch: from.pitch + (to.pitch - from.pitch) * t,
        distance: from.distance + (to.distance - from.distance) * t,
    }
}

pub fn tick_fly_speed_indicator(camera: &mut Camera, delta_seconds: f32) {
    camera.fly_speed_indicator_seconds =
        (camera.fly_speed_indicator_seconds - delta_seconds.max(0.0)).max(0.0);
}

pub fn camera_pan(camera: &mut Camera, mouse_diff: Vector2<f32>, screen_size: Vector2<f32>) {
    let right = compute_camera_right(camera);
    let up = compute_camera_up(camera);

    let fov_rad: Rad<f32> = camera.fov_y.into();
    let pan_speed = camera.distance * 2.0 * (fov_rad.0 / 2.0).tan() / screen_size.y;

    camera.pivot += right * (-mouse_diff.x * pan_speed);
    camera.pivot += up * (mouse_diff.y * pan_speed);
}

pub fn camera_zoom(
    camera: &mut Camera,
    mouse_wheel: f32,
    mouse_pos: Vector2<f32>,
    screen_size: Vector2<f32>,
) {
    let old_distance = camera.distance;
    let zoom_factor = (-mouse_wheel * 0.1).exp();
    let new_distance = (old_distance * zoom_factor).max(camera.near_plane * 2.0);

    let ndc_x = 2.0 * mouse_pos.x / screen_size.x - 1.0;
    let ndc_y = 2.0 * mouse_pos.y / screen_size.y - 1.0;

    let fov_rad: Rad<f32> = camera.fov_y.into();
    let half_height = old_distance * (fov_rad.0 / 2.0).tan();
    let aspect = screen_size.x / screen_size.y;
    let half_width = half_height * aspect;

    let right = compute_camera_right(camera);
    let up = compute_camera_up(camera);
    let cursor_world = camera.pivot + right * (ndc_x * half_width) + up * (-ndc_y * half_height);

    let percentage = 1.0 - (new_distance / old_distance);
    camera.pivot += (cursor_world - camera.pivot) * percentage;

    camera.distance = new_distance;
}

pub fn camera_reset(camera: &mut Camera) {
    camera.pivot = camera.initial_pivot;
    camera.yaw = camera.initial_yaw;
    camera.pitch = camera.initial_pitch;
    camera.distance = camera.initial_distance;

    let position = compute_camera_position(camera);
    log!(
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

pub fn camera_move_to_look_at(camera: &mut Camera, target: Vector3<f32>, offset: Vector3<f32>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn framing_distance_fills_the_vertical_fov_on_a_wide_viewport() {
        let camera = Camera::default();
        let bounds = BoundingSphere {
            center: Vector3::new(1.0, 2.0, 3.0),
            radius: 0.8,
        };
        let pose = camera_pose_framing(&camera, bounds, 16.0 / 9.0);

        let expected = 0.8 * CAMERA_FRAMING_MARGIN / (22.5f32.to_radians()).sin();
        assert!(approx(pose.distance, expected));
        assert_eq!(pose.pivot, bounds.center);
        assert_eq!(pose.yaw, camera.yaw);
    }

    #[test]
    fn framing_uses_the_horizontal_fov_on_a_tall_viewport() {
        let camera = Camera::default();
        let bounds = BoundingSphere {
            center: Vector3::new(0.0, 0.0, 0.0),
            radius: 1.0,
        };
        let wide = camera_pose_framing(&camera, bounds, 2.0);
        let tall = camera_pose_framing(&camera, bounds, 0.5);
        assert!(tall.distance > wide.distance);
    }

    #[test]
    fn eased_transition_reaches_the_target_and_clears_itself() {
        let mut camera = Camera::default();
        let target = CameraPose {
            pivot: Vector3::new(1.0, 1.0, 1.0),
            yaw: 0.3,
            pitch: 0.2,
            distance: 2.0,
        };
        camera_move_to_pose(&mut camera, target, CameraMotion::Eased);
        advance_camera_transition(&mut camera, CAMERA_FRAMING_SECONDS * 0.5);
        assert!(camera.transition.is_some());
        assert!(camera.distance > target.distance);

        advance_camera_transition(&mut camera, CAMERA_FRAMING_SECONDS);
        assert!(camera.transition.is_none());
        assert!(approx(camera.distance, target.distance));
        assert_eq!(camera.pivot, target.pivot);
    }

    #[test]
    fn wheel_scales_fly_speed_multiplicatively_and_shows_the_indicator() {
        let mut camera = Camera::default();
        adjust_fly_speed(&mut camera, 1.0);
        assert!(approx(
            camera.fly_speed_scale,
            Camera::DEFAULT_FLY_SPEED_SCALE * FLY_SPEED_WHEEL_FACTOR
        ));
        adjust_fly_speed(&mut camera, -1.0);
        assert!(approx(
            camera.fly_speed_scale,
            Camera::DEFAULT_FLY_SPEED_SCALE
        ));
        assert_eq!(
            camera.fly_speed_indicator_seconds,
            FLY_SPEED_INDICATOR_SECONDS
        );

        tick_fly_speed_indicator(&mut camera, 10.0);
        assert_eq!(camera.fly_speed_indicator_seconds, 0.0);
    }

    #[test]
    fn input_cancels_a_running_transition() {
        let mut camera = Camera::default();
        let current_pose = camera.pose();
        camera_move_to_pose(&mut camera, current_pose, CameraMotion::Eased);
        let fly = CameraFlyInput::default();
        camera_input_system_inner(
            &mut camera,
            false,
            false,
            false,
            &fly,
            1.0,
            [0.0, 0.0],
            [10.0, 10.0],
            [100.0, 100.0],
        );
        assert!(camera.transition.is_none());
    }
}
