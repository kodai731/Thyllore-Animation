use glam::{vec3, vec4, Mat4, Quat, Vec2, Vec3};

pub fn view(camera_pos: Vec3, direction: Vec3, up: Vec3) -> Mat4 {
    glam::camera::rh::view::look_to_mat4(camera_pos, direction, up)
}

/// Vulkan clip space: y flipped, depth 0..1.
pub fn perspective(fovy_radians: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fovy_radians / 2.0).tan();

    Mat4::from_cols_array(&[
        f / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        -f,
        0.0,
        0.0,
        0.0,
        0.0,
        far / (near - far),
        -1.0,
        0.0,
        0.0,
        (near * far) / (near - far),
        0.0,
    ])
}

/// Vulkan clip space with reversed depth: near maps to 1, infinity to 0.
pub fn perspective_infinite_reverse(fovy_radians: f32, aspect: f32, near: f32) -> Mat4 {
    let f = 1.0 / (fovy_radians / 2.0).tan();

    Mat4::from_cols_array(&[
        f / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        -f,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        -1.0,
        0.0,
        0.0,
        near,
        0.0,
    ])
}

/// Blender / FBX Z-up to engine Y-up: engine y = source z, engine z = -source y.
pub const BLENDER_TO_ENGINE: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
]);

pub const ENGINE_TO_BLENDER: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
]);

/// Row-major rows of `BLENDER_TO_ENGINE`, the layout Python-side matrices use.
pub fn blender_to_engine_rows() -> [[f32; 4]; 4] {
    BLENDER_TO_ENGINE.transpose().to_cols_array_2d()
}

pub fn blender_to_engine_point(point: Vec3) -> Vec3 {
    (BLENDER_TO_ENGINE * point.extend(1.0)).truncate()
}

pub fn engine_to_blender_point(point: Vec3) -> Vec3 {
    (ENGINE_TO_BLENDER * point.extend(1.0)).truncate()
}

/// Rotation of a quaternion by the same axis swap: (w, x, y, z) in Blender becomes (w, x, z, -y).
pub fn blender_to_engine_quaternion(q: Quat) -> Quat {
    Quat::from_xyzw(q.x, q.z, -q.y, q.w)
}

pub fn engine_to_blender_quaternion(q: Quat) -> Quat {
    Quat::from_xyzw(q.x, -q.z, q.y, q.w)
}

/// ワールド座標系のY軸（上向き）
pub fn world_y_axis() -> Vec3 {
    vec3(0.0, 1.0, 0.0)
}

/// ワールド座標系のY軸負方向（下向き）
pub fn world_y_down() -> Vec3 {
    vec3(0.0, -1.0, 0.0)
}

pub fn screen_to_world_ray(
    screen_pos: Vec2,
    screen_size: Vec2,
    view_matrix: Mat4,
    proj_matrix: Mat4,
) -> (Vec3, Vec3) {
    let ndc_x = (2.0 * screen_pos.x) / screen_size.x - 1.0;
    let ndc_y = (2.0 * screen_pos.y) / screen_size.y - 1.0;

    let clip_near = vec4(ndc_x, ndc_y, 1.0, 1.0);
    let clip_far = vec4(ndc_x, ndc_y, 0.0, 1.0);

    let view_proj_inverse = match (proj_matrix * view_matrix).try_inverse() {
        Some(inv) => inv,
        None => return (vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, -1.0)),
    };

    let world_near_4 = view_proj_inverse * clip_near;
    let world_far_4 = view_proj_inverse * clip_far;

    let ray_origin = vec3(
        world_near_4.x / world_near_4.w,
        world_near_4.y / world_near_4.w,
        world_near_4.z / world_near_4.w,
    );

    let ray_direction = if world_far_4.w.abs() < f32::EPSILON {
        vec3(world_far_4.x, world_far_4.y, world_far_4.z).normalize()
    } else {
        let world_far = vec3(
            world_far_4.x / world_far_4.w,
            world_far_4.y / world_far_4.w,
            world_far_4.z / world_far_4.w,
        );
        (world_far - ray_origin).normalize()
    };

    (ray_origin, ray_direction)
}

pub fn ray_to_point_distance(ray_origin: Vec3, ray_direction: Vec3, point: Vec3) -> f32 {
    let to_point = point - ray_origin;
    let projection = to_point.dot(ray_direction);
    let closest_point = ray_origin + ray_direction * projection;
    (point - closest_point).length()
}

pub fn ray_to_line_segment_distance(
    ray_origin: Vec3,
    ray_direction: Vec3,
    line_start: Vec3,
    line_end: Vec3,
) -> f32 {
    let line_dir = (line_end - line_start).normalize();
    let w0 = ray_origin - line_start;

    let a = ray_direction.dot(ray_direction);
    let b = ray_direction.dot(line_dir);
    let c = line_dir.dot(line_dir);
    let d = ray_direction.dot(w0);
    let e = line_dir.dot(w0);

    let denom = a * c - b * b;

    let (s, t) = if denom.abs() < f32::EPSILON {
        (0.0, e / c)
    } else {
        let s = (b * e - c * d) / denom;
        let t = (a * e - b * d) / denom;
        (s, t)
    };

    let t_clamped = t.max(0.0).min((line_end - line_start).length());

    let point_on_ray = ray_origin + ray_direction * s;
    let point_on_line = line_start + line_dir * t_clamped;

    (point_on_ray - point_on_line).length()
}

pub fn ray_plane_intersection(
    ray_origin: Vec3,
    ray_direction: Vec3,
    plane_point: Vec3,
    plane_normal: Vec3,
) -> Option<Vec3> {
    let denom = plane_normal.dot(ray_direction);

    if denom.abs() < f32::EPSILON {
        return None;
    }

    let t = (plane_point - ray_origin).dot(plane_normal) / denom;

    if t < 0.0 {
        return None;
    }

    Some(ray_origin + ray_direction * t)
}

pub fn get_camera_axes_from_view(view_matrix: Mat4) -> (Vec3, Vec3, Vec3) {
    let view_inverse = match view_matrix.try_inverse() {
        Some(inv) => inv,
        None => {
            return (
                vec3(1.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
                vec3(0.0, 0.0, -1.0),
            )
        }
    };

    let camera_right = vec3(
        view_inverse.x_axis.x,
        view_inverse.y_axis.x,
        view_inverse.z_axis.x,
    );
    let camera_up = vec3(
        view_inverse.x_axis.y,
        view_inverse.y_axis.y,
        view_inverse.z_axis.y,
    );
    let camera_forward = -vec3(
        view_inverse.x_axis.z,
        view_inverse.y_axis.z,
        view_inverse.z_axis.z,
    );

    (camera_right, camera_up, camera_forward)
}

pub fn world_to_screen(
    world_pos: Vec3,
    screen_size: Vec2,
    view_matrix: Mat4,
    proj_matrix: Mat4,
) -> Option<Vec2> {
    let clip_pos = proj_matrix * view_matrix * vec4(world_pos.x, world_pos.y, world_pos.z, 1.0);

    if clip_pos.w <= 0.0 {
        return None;
    }

    let ndc_x = clip_pos.x / clip_pos.w;
    let ndc_y = clip_pos.y / clip_pos.w;

    let screen_x = (ndc_x + 1.0) * 0.5 * screen_size.x;
    let screen_y = (ndc_y + 1.0) * 0.5 * screen_size.y;

    Some(Vec2::new(screen_x, screen_y))
}

pub fn ray_to_triangle_intersection(
    ray_origin: Vec3,
    ray_direction: Vec3,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) -> Option<f32> {
    ray_to_triangle_barycentric(ray_origin, ray_direction, v0, v1, v2).map(|(t, _, _)| t)
}

pub fn ray_to_triangle_barycentric(
    ray_origin: Vec3,
    ray_direction: Vec3,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) -> Option<(f32, f32, f32)> {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let h = ray_direction.cross(edge2);
    let a = edge1.dot(h);

    if a.abs() < f32::EPSILON {
        return None;
    }

    let f = 1.0 / a;
    let s = ray_origin - v0;
    let u = f * s.dot(h);

    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = s.cross(edge1);
    let v = f * ray_direction.dot(q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * edge2.dot(q);

    if t < f32::EPSILON {
        return None;
    }

    Some((t, u, v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{vec3, vec4, Vec3};

    #[test]
    fn test_blender_to_engine_maps_z_up_to_y_up() {
        let blender_up = vec4(0.0, 0.0, 1.0, 0.0);
        let engine_up = BLENDER_TO_ENGINE * blender_up;
        assert_eq!(engine_up, vec4(0.0, 1.0, 0.0, 0.0));
    }

    #[test]
    fn test_point_conversion_round_trips() {
        let blender = vec3(0.3, -1.2, 1.7);
        let engine = blender_to_engine_point(blender);
        assert_eq!(engine, vec3(0.3, 1.7, 1.2));
        assert_eq!(engine_to_blender_point(engine), blender);
    }

    #[test]
    fn test_quaternion_conversion_matches_the_matrix() {
        let blender = Quat::from_rotation_z(f32::to_radians(90.0));
        let engine = blender_to_engine_quaternion(blender);
        let rotated_by_quaternion = engine * vec3(1.0, 0.0, 0.0);
        let rotated_in_blender = blender * vec3(1.0, 0.0, 0.0);
        let expected = blender_to_engine_point(rotated_in_blender);
        assert!((rotated_by_quaternion - expected).length() < 1e-6);
        assert_eq!(engine_to_blender_quaternion(engine), blender);
    }

    #[test]
    fn test_world_axes() {
        assert_eq!(world_y_axis(), vec3(0.0, 1.0, 0.0));
        assert_eq!(world_y_down(), vec3(0.0, -1.0, 0.0));
    }

    #[test]
    fn test_perspective_y_flip() {
        let proj = perspective(f32::to_radians(45.0), 1.0, 0.1, 100.0);
        assert!(proj.y_axis.y < 0.0);
    }

    #[test]
    fn test_perspective_infinite_reverse_y_flip() {
        let proj = perspective_infinite_reverse(f32::to_radians(45.0), 1.0, 0.1);
        assert!(proj.y_axis.y < 0.0);
    }

    #[test]
    fn test_perspective_infinite_reverse_near_maps_to_one() {
        let near = 0.1;
        let proj = perspective_infinite_reverse(f32::to_radians(45.0), 1.0, near);
        let near_point = proj * vec4(0.0, 0.0, -near, 1.0);
        let ndc_z = near_point.z / near_point.w;
        assert!(
            (ndc_z - 1.0).abs() < 1e-5,
            "near should map to ndc_z=1.0, got {}",
            ndc_z
        );
    }

    #[test]
    fn test_perspective_infinite_reverse_far_maps_to_zero() {
        let proj = perspective_infinite_reverse(f32::to_radians(45.0), 1.0, 0.1);
        let far_point = proj * vec4(0.0, 0.0, -100000.0, 1.0);
        let ndc_z = far_point.z / far_point.w;
        assert!(
            ndc_z.abs() < 0.01,
            "infinity should map to ndc_z≈0.0, got {}",
            ndc_z
        );
    }

    #[test]
    fn test_right_handed() {
        let x = vec3(1.0, 0.0, 0.0);
        let y = world_y_axis();
        let z = vec3(0.0, 0.0, 1.0);
        assert!((x.cross(y) - z).length() < 1e-5);
    }

    #[test]
    fn test_view_matrix() {
        let camera_pos = Vec3::new(0.0, 0.0, -5.0);
        let direction = Vec3::new(0.0, 0.0, 1.0);
        let up = Vec3::new(0.0, 1.0, 0.0);

        let view_matrix = view(camera_pos, direction, up);

        assert_eq!(view_matrix.w_axis.w, 1.0);
        assert!(view_matrix
            .transform_point3(camera_pos)
            .abs_diff_eq(Vec3::ZERO, 1e-6));
        assert!((view_matrix.transform_point3(camera_pos + direction)).abs_diff_eq(-Vec3::Z, 1e-6));
    }

    #[test]
    fn test_ray_triangle_intersection_hit() {
        let origin = vec3(0.0, 0.0, -1.0);
        let direction = vec3(0.0, 0.0, 1.0);
        let v0 = vec3(-1.0, -1.0, 0.0);
        let v1 = vec3(1.0, -1.0, 0.0);
        let v2 = vec3(0.0, 1.0, 0.0);

        let result = ray_to_triangle_intersection(origin, direction, v0, v1, v2);
        assert!(result.is_some());
        assert!((result.unwrap() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_ray_triangle_intersection_parallel() {
        let origin = vec3(0.0, 0.0, -1.0);
        let direction = vec3(1.0, 0.0, 0.0);
        let v0 = vec3(-1.0, -1.0, 0.0);
        let v1 = vec3(1.0, -1.0, 0.0);
        let v2 = vec3(0.0, 1.0, 0.0);

        let result = ray_to_triangle_intersection(origin, direction, v0, v1, v2);
        assert!(result.is_none());
    }

    #[test]
    fn test_ray_triangle_intersection_behind() {
        let origin = vec3(0.0, 0.0, 1.0);
        let direction = vec3(0.0, 0.0, 1.0);
        let v0 = vec3(-1.0, -1.0, 0.0);
        let v1 = vec3(1.0, -1.0, 0.0);
        let v2 = vec3(0.0, 1.0, 0.0);

        let result = ray_to_triangle_intersection(origin, direction, v0, v1, v2);
        assert!(result.is_none());
    }

    #[test]
    fn test_screen_to_world_ray_infinite_reverse_z_no_nan() {
        let camera_pos = vec3(0.0, 1.0, -3.0);
        let direction = vec3(0.0, 0.0, 1.0);
        let up = vec3(0.0, 1.0, 0.0);

        let view_mat = view(camera_pos, direction, up);
        let proj_mat = perspective_infinite_reverse(f32::to_radians(45.0), 16.0 / 9.0, 0.1);

        let screen_center = Vec2::new(400.0, 300.0);
        let screen_size = Vec2::new(800.0, 600.0);

        let (origin, dir) = screen_to_world_ray(screen_center, screen_size, view_mat, proj_mat);

        assert!(!origin.x.is_nan() && !origin.y.is_nan() && !origin.z.is_nan());
        assert!(!dir.x.is_nan() && !dir.y.is_nan() && !dir.z.is_nan());
        assert!((dir.length() - 1.0).abs() < 1e-4);
    }
}
