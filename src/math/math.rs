use cgmath::num_traits::real::Real;
pub use cgmath::Quaternion;
pub use cgmath::Rad;
pub use cgmath::{point3, Deg, InnerSpace, MetricSpace, Vector2};
pub use cgmath::{prelude::*, Vector3};
pub use cgmath::{vec2, vec3, vec4};
use cgmath::{Matrix3, Matrix4};
use std::f32::EPSILON;
use std::ops::{Add, AddAssign, Deref, DerefMut, Mul, Neg};

#[derive(Copy, Clone, Debug)]
pub struct Vec2(cgmath::Vector2<f32>);
impl Default for Vec2 {
    fn default() -> Self {
        Self(cgmath::Vector2::new(0.0, 0.0))
    }
}

impl Deref for Vec2 {
    type Target = cgmath::Vector2<f32>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for Vec2 {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(self.0.neg())
    }
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self(cgmath::Vector2::new(x, y))
    }

    pub fn distance(&self, other: Self) -> f32 {
        cgmath::Vector2::distance(self.0, other.0)
    }

    pub fn new_array(array: [f32; 2]) -> Self {
        Self(cgmath::Vector2::new(array[0], array[1]))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Vec3(cgmath::Vector3<f32>);
impl Default for Vec3 {
    fn default() -> Self {
        Self(cgmath::Vector3::new(0.0, 0.0, 0.0))
    }
}
impl Deref for Vec3 {
    type Target = cgmath::Vector3<f32>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Vec3 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(cgmath::Vector3::new(x, y, z))
    }

    pub fn new_array(p: [f32; 3]) -> Self {
        Self(cgmath::Vector3::new(p[0], p[1], p[2]))
    }
}

impl PartialEq for Vec3 {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

pub type Vector4 = cgmath::Vector4<f32>;

#[derive(Copy, Clone, Debug)]
pub struct Vec4(cgmath::Vector4<f32>);
impl Default for Vec4 {
    fn default() -> Self {
        Self(cgmath::Vector4::new(0.0, 0.0, 0.0, 0.0))
    }
}
impl Deref for Vec4 {
    type Target = cgmath::Vector4<f32>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for Vec4 {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Neg for Vec4 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl Vec4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self(cgmath::Vector4::new(x, y, z, w))
    }
}

pub type Mat3 = cgmath::Matrix3<f32>;
pub type Mat4 = cgmath::Matrix4<f32>;

pub fn vec3_from_array(a: [f32; 3]) -> Vector3<f32> {
    vec3(a[0], a[1], a[2])
}

pub fn array3_from_vec(v: Vector3<f32>) -> [f32; 3] {
    [v.x, v.y, v.z]
}

pub fn vec2_from_array(a: [f32; 2]) -> Vector2<f32> {
    vec2(a[0], a[1])
}

pub fn array2_from_vec(v: Vector2<f32>) -> [f32; 2] {
    [v.x, v.y]
}

pub fn vec4_from_array(a: [f32; 4]) -> cgmath::Vector4<f32> {
    cgmath::Vector4::new(a[0], a[1], a[2], a[3])
}

pub fn array4_from_vec(v: cgmath::Vector4<f32>) -> [f32; 4] {
    [v.x, v.y, v.z, v.w]
}

pub fn array_from_mat4(m: Mat4) -> [[f32; 4]; 4] {
    [
        array4_from_vec(m.x),
        array4_from_vec(m.y),
        array4_from_vec(m.z),
        array4_from_vec(m.w),
    ]
}

pub fn mat4_from_array(a: [[f32; 4]; 4]) -> Mat4 {
    Mat4::from_cols(a[0].into(), a[1].into(), a[2].into(), a[3].into())
}

pub fn fix_coord() -> Mat4 {
    Matrix4::from_cols(
        Vector4::new(1.0, 0.0, 0.0, 0.0),  // X ← X
        Vector4::new(0.0, 0.0, 1.0, 0.0),  // Y ← Z
        Vector4::new(0.0, 1.0, 0.0, 0.0), // Z ← -Y
        Vector4::new(0.0, 0.0, 0.0, 1.0),
    )
}

pub fn decompose(m: &Matrix4<f32>) -> (Vector3<f32>, Quaternion<f32>, Vector3<f32>) {
    let translation = Vector3::new(m.w.x, m.w.y, m.w.z);

    let scale_x = Vector3::new(m.x.x, m.x.y, m.x.z).magnitude();
    let scale_y = Vector3::new(m.y.x, m.y.y, m.y.z).magnitude();
    let scale_z = Vector3::new(m.z.x, m.z.y, m.z.z).magnitude();
    let scale = Vector3::new(scale_x, scale_y, scale_z);

    let rot_x = Vector3::new(m.x.x, m.x.y, m.x.z) / scale_x;
    let rot_y = Vector3::new(m.y.x, m.y.y, m.y.z) / scale_y;
    let rot_z = Vector3::new(m.z.x, m.z.y, m.z.z) / scale_z;

    let rot_matrix = Matrix3::from_cols(rot_x, rot_y, rot_z);

    let rotation = Quaternion::from(rot_matrix);

    (translation, rotation, scale)
}

pub fn swap(q: &Quaternion<f32>) -> Quaternion<f32> {
    Quaternion::new(q.s, q.v[0], q.v[1], q.v[2])
}

pub unsafe fn rodrigues(
    rotate: &mut cgmath::Matrix3<f32>,
    c: f32,
    s: f32,
    n: &cgmath::Vector3<f32>,
) -> anyhow::Result<()> {
    let ac = 1.0 - c;
    let xyac = n.x * n.y * ac;
    let yzac = n.y * n.z * ac;
    let zxac = n.x * n.z * ac;
    let xs = n.x * s;
    let ys = n.y * s;
    let zs = n.z * s;
    // rotate = glm::mat3(c + n.x * n.x * ac, n.x * n.y * ac + n.z * s, n.z * n.x * ac - n.y * s,
    //     n.x * n.y * ac - n.z * s, c + n.y * n.y * ac, n.y * n.z * ac + n.x * s,
    //     n.z * n.x * ac + n.y * s, n.y * n.z * ac - n.x * s, c + n.z * n.z * ac);
    *rotate = cgmath::Matrix3::new(
        c + n.x * n.x * ac,
        xyac + zs,
        zxac - ys,
        xyac - zs,
        c + n.y * n.y * ac,
        yzac + xs,
        zxac + ys,
        yzac - xs,
        c + n.z * n.z * ac,
    );
    Ok(())
}

pub unsafe fn view(
    camera_pos: cgmath::Vector3<f32>,
    direction: cgmath::Vector3<f32>,
    up: cgmath::Vector3<f32>,
) -> cgmath::Matrix4<f32> {
    let n_z = cgmath::Vector3::normalize(direction);
    let n_x = cgmath::Vector3::normalize(cgmath::Vector3::cross(up, n_z));
    let n_y = cgmath::Vector3::cross(n_x, n_z);
    let orientation = cgmath::Matrix4::new(
        n_x.x, n_y.x, n_z.x, 0.0, n_x.y, n_y.y, n_z.y, 0.0, n_x.z, n_y.z, n_z.z, 0.0, 0.0, 0.0,
        0.0, 1.0,
    );
    let translate = cgmath::Matrix4::new(
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        -camera_pos.x,
        -camera_pos.y,
        -camera_pos.z,
        1.0,
    );
    return orientation * translate;
}

pub fn approx_equal_array3(a: &[f32; 3], b: &[f32; 3]) -> bool {
    (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5 && (a[2] - b[2]).abs() < 1e-5
}

pub fn approx_equal_array4(a: &[f32; 4], b: &[f32; 4]) -> bool {
    (a[0] - b[0]).abs() < 1e-5
        && (a[1] - b[1]).abs() < 1e-3
        && (a[2] - b[2]).abs() < 1e-3
        && (a[3] - b[3]).abs() < 1e-3
}

pub fn approx_equal_vec4(a: &Vector4, b: &Vector4) -> bool {
    (a.x - b.x).abs() < 1e-3
        && (a.y - b.y).abs() < 1e-3
        && (a.z - b.z).abs() < 1e-3
        && (a.w - b.w).abs() < 1e-3
}

pub fn approx_equal_mat4(a: &Mat4, b: &Mat4) -> bool {
    approx_equal_vec4(&a.x, &b.x)
        && approx_equal_vec4(&a.y, &b.y)
        && approx_equal_vec4(&a.z, &b.z)
        && approx_equal_vec4(&a.w, &b.w)
}

pub fn screen_to_world_ray(
    screen_pos: Vector2<f32>,
    screen_size: Vector2<f32>,
    view_matrix: Mat4,
    proj_matrix: Mat4,
) -> (Vector3<f32>, Vector3<f32>) {
    let ndc_x = (2.0 * screen_pos.x) / screen_size.x - 1.0;
    let ndc_y = 1.0 - (2.0 * screen_pos.y) / screen_size.y;

    let clip_near = cgmath::vec4(ndc_x, ndc_y, -1.0, 1.0);
    let clip_far = cgmath::vec4(ndc_x, ndc_y, 1.0, 1.0);

    let view_proj_inverse = (proj_matrix * view_matrix).invert().unwrap();

    let world_near_4 = view_proj_inverse * clip_near;
    let world_far_4 = view_proj_inverse * clip_far;

    let world_near = vec3(
        world_near_4.x / world_near_4.w,
        world_near_4.y / world_near_4.w,
        world_near_4.z / world_near_4.w,
    );
    let world_far = vec3(
        world_far_4.x / world_far_4.w,
        world_far_4.y / world_far_4.w,
        world_far_4.z / world_far_4.w,
    );

    let ray_origin = world_near;
    let ray_direction = (world_far - world_near).normalize();

    (ray_origin, ray_direction)
}

pub fn ray_to_point_distance(
    ray_origin: Vector3<f32>,
    ray_direction: Vector3<f32>,
    point: Vector3<f32>,
) -> f32 {
    let to_point = point - ray_origin;
    let projection = to_point.dot(ray_direction);
    let closest_point = ray_origin + ray_direction * projection;
    (point - closest_point).magnitude()
}

pub fn ray_to_line_segment_distance(
    ray_origin: Vector3<f32>,
    ray_direction: Vector3<f32>,
    line_start: Vector3<f32>,
    line_end: Vector3<f32>,
) -> f32 {
    let line_dir = (line_end - line_start).normalize();
    let w0 = ray_origin - line_start;

    let a = ray_direction.dot(ray_direction);
    let b = ray_direction.dot(line_dir);
    let c = line_dir.dot(line_dir);
    let d = ray_direction.dot(w0);
    let e = line_dir.dot(w0);

    let denom = a * c - b * b;

    let (s, t) = if denom.abs() < EPSILON {
        (0.0, e / c)
    } else {
        let s = (b * e - c * d) / denom;
        let t = (a * e - b * d) / denom;
        (s, t)
    };

    let t_clamped = t.max(0.0).min((line_end - line_start).magnitude());

    let point_on_ray = ray_origin + ray_direction * s;
    let point_on_line = line_start + line_dir * t_clamped;

    (point_on_ray - point_on_line).magnitude()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec2_new() {
        let v = Vec2::new(1.0, 2.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
    }

    #[test]
    fn test_vec2_default() {
        let v = Vec2::default();
        assert_eq!(v.x, 0.0);
        assert_eq!(v.y, 0.0);
    }

    #[test]
    fn test_vec2_distance() {
        let v1 = Vec2::new(0.0, 0.0);
        let v2 = Vec2::new(3.0, 4.0);
        let distance = v1.distance(v2);
        assert!((distance - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_vec2_new_array() {
        let v = Vec2::new_array([3.5, 4.5]);
        assert_eq!(v.x, 3.5);
        assert_eq!(v.y, 4.5);
    }

    #[test]
    fn test_vec2_neg() {
        let v = Vec2::new(1.0, -2.0);
        let negated = -v;
        assert_eq!(negated.x, -1.0);
        assert_eq!(negated.y, 2.0);
    }

    #[test]
    fn test_vec3_new() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn test_vec3_default() {
        let v = Vec3::default();
        assert_eq!(v.x, 0.0);
        assert_eq!(v.y, 0.0);
        assert_eq!(v.z, 0.0);
    }

    #[test]
    fn test_vec3_new_array() {
        let v = Vec3::new_array([1.5, 2.5, 3.5]);
        assert_eq!(v.x, 1.5);
        assert_eq!(v.y, 2.5);
        assert_eq!(v.z, 3.5);
    }

    #[test]
    fn test_vec3_add() {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);
        let result = v1 + v2;
        assert_eq!(result.x, 5.0);
        assert_eq!(result.y, 7.0);
        assert_eq!(result.z, 9.0);
    }

    #[test]
    fn test_vec3_mul_scalar() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let result = v * 2.0;
        assert_eq!(result.x, 2.0);
        assert_eq!(result.y, 4.0);
        assert_eq!(result.z, 6.0);
    }

    #[test]
    fn test_vec3_neg() {
        let v = Vec3::new(1.0, -2.0, 3.0);
        let negated = -v;
        assert_eq!(negated.x, -1.0);
        assert_eq!(negated.y, 2.0);
        assert_eq!(negated.z, -3.0);
    }

    #[test]
    fn test_vec3_add_assign() {
        let mut v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);
        v1 += v2;
        assert_eq!(v1.x, 5.0);
        assert_eq!(v1.y, 7.0);
        assert_eq!(v1.z, 9.0);
    }

    #[test]
    fn test_vec4_new() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
        assert_eq!(v.w, 4.0);
    }

    #[test]
    fn test_vec4_default() {
        let v = Vec4::default();
        assert_eq!(v.x, 0.0);
        assert_eq!(v.y, 0.0);
        assert_eq!(v.z, 0.0);
        assert_eq!(v.w, 0.0);
    }

    #[test]
    fn test_vec4_neg() {
        let v = Vec4::new(1.0, -2.0, 3.0, -4.0);
        let negated = -v;
        assert_eq!(negated.x, -1.0);
        assert_eq!(negated.y, 2.0);
        assert_eq!(negated.z, -3.0);
        assert_eq!(negated.w, 4.0);
    }

    #[test]
    fn test_vec3_from_array() {
        let v = vec3_from_array([1.0, 2.0, 3.0]);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn test_array3_from_vec() {
        let v = vec3(1.5, 2.5, 3.5);
        let arr = array3_from_vec(v);
        assert_eq!(arr, [1.5, 2.5, 3.5]);
    }

    #[test]
    fn test_vec2_from_array() {
        let v = vec2_from_array([1.0, 2.0]);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
    }

    #[test]
    fn test_array2_from_vec() {
        let v = vec2(1.5, 2.5);
        let arr = array2_from_vec(v);
        assert_eq!(arr, [1.5, 2.5]);
    }

    #[test]
    fn test_vec4_from_array() {
        let v = vec4_from_array([1.0, 2.0, 3.0, 4.0]);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
        assert_eq!(v.w, 4.0);
    }

    #[test]
    fn test_array4_from_vec() {
        let v = cgmath::Vector4::new(1.5, 2.5, 3.5, 4.5);
        let arr = array4_from_vec(v);
        assert_eq!(arr, [1.5, 2.5, 3.5, 4.5]);
    }

    #[test]
    fn test_mat4_array_conversion() {
        let original = Mat4::from_cols(
            cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
            cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
            cgmath::Vector4::new(0.0, 0.0, 1.0, 0.0),
            cgmath::Vector4::new(0.0, 0.0, 0.0, 1.0),
        );

        let arr = array_from_mat4(original);
        let converted = mat4_from_array(arr);

        assert!(approx_equal_mat4(&original, &converted));
    }

    #[test]
    fn test_approx_equal_array3() {
        let a = [1.0, 2.0, 3.0];
        let b = [1.000001, 2.000001, 3.000001];
        assert!(approx_equal_array3(&a, &b));

        let c = [1.0, 2.0, 3.0];
        let d = [1.1, 2.0, 3.0];
        assert!(!approx_equal_array3(&c, &d));
    }

    #[test]
    fn test_approx_equal_array4() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [1.000001, 2.0001, 3.0001, 4.0001];
        assert!(approx_equal_array4(&a, &b));
    }

    #[test]
    fn test_approx_equal_vec4() {
        let a = cgmath::Vector4::new(1.0, 2.0, 3.0, 4.0);
        let b = cgmath::Vector4::new(1.0001, 2.0001, 3.0001, 4.0001);
        assert!(approx_equal_vec4(&a, &b));
    }

    #[test]
    fn test_fix_coord() {
        let m = fix_coord();
        assert_eq!(m.x.x, 1.0);
        assert_eq!(m.y.z, 1.0);
        assert_eq!(m.z.y, 1.0);
        assert_eq!(m.w.w, 1.0);
    }

    #[test]
    fn test_swap_quaternion() {
        let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let swapped = swap(&q);
        assert_eq!(swapped.s, 1.0);
        assert_eq!(swapped.v.x, 2.0);
        assert_eq!(swapped.v.y, 3.0);
        assert_eq!(swapped.v.z, 4.0);
    }

    #[test]
    fn test_decompose_identity() {
        let identity = Matrix4::from_scale(1.0);
        let (translation, rotation, scale) = decompose(&identity);

        assert!((translation.x).abs() < 1e-5);
        assert!((translation.y).abs() < 1e-5);
        assert!((translation.z).abs() < 1e-5);

        assert!((scale.x - 1.0).abs() < 1e-5);
        assert!((scale.y - 1.0).abs() < 1e-5);
        assert!((scale.z - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_rodrigues_rotation() {
        unsafe {
            let mut rotate = Matrix3::from_value(0.0);
            let angle = std::f32::consts::PI / 2.0;
            let c = angle.cos();
            let s = angle.sin();
            let n = Vector3::new(0.0, 0.0, 1.0);

            let result = rodrigues(&mut rotate, c, s, &n);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_view_matrix() {
        unsafe {
            let camera_pos = Vector3::new(0.0, 0.0, -5.0);
            let direction = Vector3::new(0.0, 0.0, 1.0);
            let up = Vector3::new(0.0, 1.0, 0.0);

            let view_matrix = view(camera_pos, direction, up);

            assert_eq!(view_matrix.w.w, 1.0);
        }
    }

    #[test]
    fn test_vec3_equality() {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(1.0, 2.0, 3.0);
        let v3 = Vec3::new(1.0, 2.0, 3.1);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_vec2_equality() {
        let v1 = Vec2::new(1.0, 2.0);
        let v2 = Vec2::new(1.0, 2.0);
        let v3 = Vec2::new(1.0, 2.1);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_vec4_equality() {
        let v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v3 = Vec4::new(1.0, 2.0, 3.0, 4.1);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }
}
