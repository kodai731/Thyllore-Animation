use crate::volume::knots::RayKnots;
use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::biweight_sphere_piece_integral;

// Mirror of shaders/include/volume_puffs.glsl.

pub const PUFFS_PER_RAY: usize = 20;
const EMPTY_INTERVAL_EPSILON: f32 = 1e-6;

/// Puffs a ray enters, with their entry and exit ray parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayPuffs {
    pub count: usize,
    pub index: [usize; PUFFS_PER_RAY],
    pub enter: [f32; PUFFS_PER_RAY],
    pub exit: [f32; PUFFS_PER_RAY],
}

impl Default for RayPuffs {
    fn default() -> Self {
        Self {
            count: 0,
            index: [0; PUFFS_PER_RAY],
            enter: [0.0; PUFFS_PER_RAY],
            exit: [0.0; PUFFS_PER_RAY],
        }
    }
}

impl RayPuffs {
    /// Adds the entry and exit of every sphere (xyz centre, w radius) the ray crosses as knots.
    pub fn collect(
        spheres: &[[f32; 4]],
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        t_near: f32,
        t_far: f32,
        knots: &mut RayKnots,
    ) -> Self {
        let mut puffs = Self::default();
        let a = direction.dot(direction);
        for (i, sphere) in spheres.iter().enumerate() {
            if puffs.count >= PUFFS_PER_RAY {
                break;
            }
            let radius = sphere[3];
            if radius <= 0.0 {
                continue;
            }
            let dx = origin - Vector3::new(sphere[0], sphere[1], sphere[2]);
            let b = 2.0 * dx.dot(direction);
            let c = dx.dot(dx) - radius * radius;
            let discriminant = b * b - 4.0 * a * c;
            if discriminant <= 0.0 {
                continue;
            }
            let sqrt_discriminant = discriminant.sqrt();
            let t0 = (-b - sqrt_discriminant) / (2.0 * a);
            let t1 = (-b + sqrt_discriminant) / (2.0 * a);
            if t1 <= t_near || t0 >= t_far {
                continue;
            }
            knots.push(t0, t_near, t_far);
            knots.push(t1, t_near, t_far);
            puffs.index[puffs.count] = i;
            puffs.enter[puffs.count] = t0;
            puffs.exit[puffs.count] = t1;
            puffs.count += 1;
        }
        puffs
    }

    pub fn piece_holds(&self, s0: f32, s1: f32) -> bool {
        let s_mid = 0.5 * (s0 + s1);
        (0..self.count).any(|k| s_mid > self.enter[k] && s_mid < self.exit[k])
    }

    /// Unit-strength integral of every puff enclosing the piece [s0, s1], which must not cross a knot.
    pub fn piece_integral(
        &self,
        spheres: &[[f32; 4]],
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        s0: f32,
        s1: f32,
    ) -> f32 {
        let length = s1 - s0;
        if length <= EMPTY_INTERVAL_EPSILON || self.count == 0 {
            return 0.0;
        }
        let s_mid = 0.5 * (s0 + s1);
        let start = origin + direction * s0;

        let mut total = 0.0f32;
        for k in 0..self.count {
            if s_mid <= self.enter[k] || s_mid >= self.exit[k] {
                continue;
            }
            let sphere = &spheres[self.index[k]];
            let center = Vector3::new(sphere[0], sphere[1], sphere[2]);
            total += biweight_sphere_piece_integral(center, sphere[3], start, direction, length);
        }
        length * total
    }
}
