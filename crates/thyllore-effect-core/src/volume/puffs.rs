use crate::volume::knots::RayKnots;
use crate::volume::shell::VolumeShell;
use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::{biweight, biweight_sphere_piece_integral};

// Mirror of shaders/include/volume_puffs.slang.

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

    /// Optical depth at `sigma` per unit kernel of every puff enclosing the piece [s0, s1].
    pub fn piece_optical_depth(
        &self,
        spheres: &[[f32; 4]],
        sigma: f32,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        s0: f32,
        s1: f32,
    ) -> f32 {
        (sigma * self.piece_integral(spheres, origin, direction, s0, s1)).max(0.0)
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

/// Shell knots plus the entry and exit of every puff the ray crosses, sorted, with the crossed puffs.
pub fn shell_and_puff_knots(
    shell: &VolumeShell,
    spheres: &[[f32; 4]],
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    t_near: f32,
    t_far: f32,
) -> (RayKnots, RayPuffs) {
    let mut knots = RayKnots::begin(t_near, t_far);
    shell.collect_knots(origin, direction, t_near, t_far, &mut knots);
    let puffs = RayPuffs::collect(spheres, origin, direction, t_near, t_far, &mut knots);
    knots.sort();
    (knots, puffs)
}

/// Sum of the biweight kernels, scaled by `strength`, of every sphere containing `point`.
pub fn puff_density(spheres: &[[f32; 4]], strength: f32, point: Vector3<f32>) -> f32 {
    let mut density = 0.0f32;
    for sphere in spheres {
        let radius = sphere[3];
        if radius <= 0.0 {
            continue;
        }
        let offset = point - Vector3::new(sphere[0], sphere[1], sphere[2]);
        let u = offset.dot(offset) / (radius * radius);
        if u < 1.0 {
            density += strength * biweight(u);
        }
    }
    density
}

/// Density of the shell wall with the puffs added inside the wall envelope.
pub fn shell_and_puff_density(
    shell: &VolumeShell,
    spheres: &[[f32; 4]],
    puff_strength: f32,
    local: Vector3<f32>,
) -> f32 {
    let h = local.y / shell.height;
    let envelope = shell.envelope_height(h);
    if envelope <= 0.0 {
        return 0.0;
    }
    let q = local.x * local.x + local.z * local.z;
    let wall = shell.wall_at(q, h);
    let puffs = puff_density(spheres, puff_strength * shell.strength, local);
    shell.sigma_t * (envelope * wall + puffs)
}
