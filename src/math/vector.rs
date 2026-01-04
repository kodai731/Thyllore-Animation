use cgmath::{vec2, vec3, InnerSpace, MetricSpace, Vector2, Vector3};
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
