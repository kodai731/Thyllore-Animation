use crate::volume::knots::{RayKnots, RAY_LINEAR_COEFFICIENT_EPSILON};
use cgmath::Vector3;
use thyllore_math_core::{
    biweight, biweight_poly, one_minus_smootherstep_poly, poly_from_quadratic,
    poly_linear_weighted_moments, poly_moments, poly_mul, poly_scale, poly_zero, Poly,
};

// Mirror of shaders/include/volume_shell.glsl.
//
// Wall of a participating medium as a compact-support shell in q = x^2 + z^2 around a radius
// linear in the normalised height h = y / height:
//   density = strength * B((q - P(h)) / width_q),  P(h) = (radius_base + radius_slope * h)^2 + radius_offset_q
// with B(u) = (1 - u^2)^2 on |u| < 1, times a smootherstep envelope fading over the top
// `top_fade` fraction of h_top. Along a ray q is quadratic and h linear in the parameter, so on
// every piece between knots the density is one polynomial integrated by exact power moments.

const EMPTY_INTERVAL_EPSILON: f32 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VolumeShell {
    pub height: f32,
    pub radius_base: f32,
    pub radius_slope: f32,
    pub radius_offset_q: f32,
    pub width_q: f32,
    pub strength: f32,
    pub h_top: f32,
    pub top_fade: f32,
    pub sigma_t: f32,
}

pub fn clamp_ray_to_cone_frustum(
    radius_base: f32,
    radius_top: f32,
    top_y: f32,
    origin: Vector3<f32>,
    direction: Vector3<f32>,
    bounds: (f32, f32),
) -> Option<(f32, f32)> {
    let (mut t_near, mut t_far) = bounds;
    let slope_per_unit_y = (radius_top - radius_base) / top_y;
    let m = radius_base + slope_per_unit_y * origin.y;
    let n = slope_per_unit_y * direction.y;
    let a = direction.x * direction.x + direction.z * direction.z - n * n;
    let b = 2.0 * (origin.x * direction.x + origin.z * direction.z - m * n);
    let c = origin.x * origin.x + origin.z * origin.z - m * m;

    if a.abs() < RAY_LINEAR_COEFFICIENT_EPSILON {
        if b.abs() < RAY_LINEAR_COEFFICIENT_EPSILON {
            if c > 0.0 {
                return None;
            }
        } else {
            let t_root = -c / b;
            if b > 0.0 {
                t_far = t_far.min(t_root);
            } else {
                t_near = t_near.max(t_root);
            }
        }
    } else {
        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            if a > 0.0 {
                return None;
            }
        } else if a > 0.0 {
            let sqrt_discriminant = discriminant.sqrt();
            let t0 = (-b - sqrt_discriminant) / (2.0 * a);
            let t1 = (-b + sqrt_discriminant) / (2.0 * a);
            t_near = t_near.max(t0.min(t1));
            t_far = t_far.min(t0.max(t1));
        }
    }

    if direction.y.abs() < RAY_LINEAR_COEFFICIENT_EPSILON {
        if origin.y < 0.0 || origin.y > top_y {
            return None;
        }
    } else {
        let t_y0 = -origin.y / direction.y;
        let t_y1 = (top_y - origin.y) / direction.y;
        t_near = t_near.max(t_y0.min(t_y1));
        t_far = t_far.min(t_y0.max(t_y1));
    }

    (t_near <= t_far).then_some((t_near, t_far))
}

impl VolumeShell {
    pub fn fade_start(&self) -> f32 {
        1.0 - self.top_fade
    }

    pub fn wall_radius(&self, h: f32) -> f32 {
        self.radius_base + self.radius_slope * h
    }

    pub fn wall_radius_sq(&self, h: f32) -> f32 {
        let radius = self.wall_radius(h);
        radius * radius + self.radius_offset_q
    }

    pub fn envelope_radius(&self, h: f32) -> f32 {
        self.wall_radius_sq(h).max(0.0).sqrt() + self.width_q.sqrt()
    }

    pub fn envelope_height(&self, h: f32) -> f32 {
        if h < 0.0 || h > self.h_top {
            return 0.0;
        }
        let normalized_height = h / self.h_top;
        let fade_start = self.fade_start();
        if normalized_height <= fade_start {
            return 1.0;
        }
        let v = (normalized_height - fade_start) / self.top_fade;
        1.0 - v * v * v * (10.0 - v * (15.0 - 6.0 * v))
    }

    pub fn wall_at(&self, q: f32, h: f32) -> f32 {
        self.strength * biweight((q - self.wall_radius_sq(h)) / self.width_q)
    }

    pub fn density_at(&self, local: Vector3<f32>) -> f32 {
        let h = local.y / self.height;
        let envelope = self.envelope_height(h);
        if envelope <= 0.0 {
            return 0.0;
        }
        let q = local.x * local.x + local.z * local.z;
        self.sigma_t * envelope * self.wall_at(q, h)
    }

    /// Clamps the ray parameter interval to the envelope cone frustum intersected with the
    /// height slab; `None` when the ray misses it.
    pub fn clamp_ray_to_cone(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        bounds: (f32, f32),
    ) -> Option<(f32, f32)> {
        clamp_ray_to_cone_frustum(
            self.envelope_radius(0.0),
            self.envelope_radius(self.h_top),
            self.h_top * self.height,
            origin,
            direction,
            bounds,
        )
    }

    /// Wall support boundaries and the envelope break of the ray, appended unsorted.
    pub fn collect_knots(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        t_near: f32,
        t_far: f32,
        knots: &mut RayKnots,
    ) {
        let q_a = direction.x * direction.x + direction.z * direction.z;
        let q_b = 2.0 * (origin.x * direction.x + origin.z * direction.z);
        let q_c = origin.x * origin.x + origin.z * origin.z;

        let inv_height = 1.0 / self.height;
        let radius_0 = self.wall_radius(origin.y * inv_height);
        let radius_1 = self.radius_slope * direction.y * inv_height;
        let delta_a = q_a - radius_1 * radius_1;
        let delta_b = q_b - 2.0 * radius_0 * radius_1;
        let delta_c = q_c - radius_0 * radius_0 - self.radius_offset_q;
        for boundary in [self.width_q, -self.width_q] {
            knots.push_quadratic_roots(delta_a, delta_b, delta_c - boundary, t_near, t_far);
        }

        if direction.y.abs() >= RAY_LINEAR_COEFFICIENT_EPSILON {
            let fade_y = self.fade_start() * self.h_top * self.height;
            knots.push((fade_y - origin.y) / direction.y, t_near, t_far);
        }
    }

    pub fn knots(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        t_near: f32,
        t_far: f32,
    ) -> RayKnots {
        let mut knots = RayKnots::begin(t_near, t_far);
        self.collect_knots(origin, direction, t_near, t_far, &mut knots);
        knots.sort();
        knots
    }

    fn envelope_poly(&self, h0: f32, h1: f32, h_mid: f32) -> Poly {
        let fade_start = self.fade_start();
        if h_mid <= fade_start {
            let mut envelope = poly_zero();
            envelope[0] = 1.0;
            return envelope;
        }
        one_minus_smootherstep_poly((h0 - fade_start) / self.top_fade, h1 / self.top_fade)
    }

    fn piece_distance_at_mid(
        &self,
        start: Vector3<f32>,
        direction: Vector3<f32>,
        length: f32,
        h_mid: f32,
    ) -> f32 {
        let mid = start + direction * (0.5 * length);
        let radius_mid = self.wall_radius(h_mid);
        (mid.x * mid.x + mid.z * mid.z - radius_mid * radius_mid - self.radius_offset_q)
            / self.width_q
    }

    /// True when the piece [s0, s1], which must not cross a knot, lies inside the wall support.
    pub fn piece_holds(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        s0: f32,
        s1: f32,
    ) -> bool {
        let length = s1 - s0;
        if length <= EMPTY_INTERVAL_EPSILON {
            return false;
        }
        let start = origin + direction * s0;
        let h_mid = (start.y + 0.5 * length * direction.y) / self.height;
        if !(0.0..=self.h_top).contains(&h_mid) {
            return false;
        }
        self.piece_distance_at_mid(start, direction, length, h_mid)
            .abs()
            < 1.0
    }

    /// Envelope times wall on the piece as a polynomial in sigma; `None` when the piece holds no wall.
    pub fn piece_poly(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        s0: f32,
        s1: f32,
    ) -> Option<Poly> {
        if !self.piece_holds(origin, direction, s0, s1) {
            return None;
        }
        let length = s1 - s0;
        let start = origin + direction * s0;
        let inv_height = 1.0 / self.height;
        let h0 = start.y * inv_height;
        let h1 = length * direction.y * inv_height;
        let h_mid = h0 + 0.5 * h1;

        let q0 = start.x * start.x + start.z * start.z;
        let q1 = 2.0 * length * (start.x * direction.x + start.z * direction.z);
        let q2 = length * length * (direction.x * direction.x + direction.z * direction.z);

        let radius_0 = self.wall_radius(h0);
        let radius_1 = self.radius_slope * h1;
        let inv_width = 1.0 / self.width_q;
        let u = poly_from_quadratic(
            (q0 - radius_0 * radius_0 - self.radius_offset_q) * inv_width,
            (q1 - 2.0 * radius_0 * radius_1) * inv_width,
            (q2 - radius_1 * radius_1) * inv_width,
        );

        let wall = biweight_poly(&u);
        let inv_h_top = 1.0 / self.h_top;
        let envelope = self.envelope_poly(h0 * inv_h_top, h1 * inv_h_top, h_mid * inv_h_top);
        let mut density = poly_mul(&envelope, &wall);
        poly_scale(&mut density, self.strength);
        Some(density)
    }

    pub fn piece_optical_depth(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        s0: f32,
        s1: f32,
    ) -> f32 {
        let Some(density) = self.piece_poly(origin, direction, s0, s1) else {
            return 0.0;
        };
        ((s1 - s0) * self.sigma_t * poly_moments(&density)).max(0.0)
    }

    /// A multiplicative modulation linear on the piece between the given end values.
    pub fn piece_optical_depth_modulated(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        s0: f32,
        s1: f32,
        modulation: (f32, f32),
    ) -> f32 {
        let Some(density) = self.piece_poly(origin, direction, s0, s1) else {
            return 0.0;
        };
        let (modulation_0, modulation_1) = modulation;
        let total = poly_linear_weighted_moments(&density, modulation_0, modulation_1);
        ((s1 - s0) * self.sigma_t * total).max(0.0)
    }

    pub fn optical_depth(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        t_near: f32,
        t_far: f32,
    ) -> f32 {
        if t_far <= t_near {
            return 0.0;
        }
        self.knots(origin, direction, t_near, t_far)
            .pieces()
            .map(|(s0, s1)| self.piece_optical_depth(origin, direction, s0, s1))
            .sum()
    }

    pub fn optical_depth_toward(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        t_max: f32,
    ) -> f32 {
        let Some((t_near, t_far)) = self.clamp_ray_to_cone(origin, direction, (0.0, t_max)) else {
            return 0.0;
        };
        self.optical_depth(origin, direction, t_near.max(0.0), t_far)
    }
}
