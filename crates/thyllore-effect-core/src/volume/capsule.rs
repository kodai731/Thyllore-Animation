use crate::volume::knots::{RayKnots, RAY_LINEAR_COEFFICIENT_EPSILON};
use cgmath::{InnerSpace, Vector3, Zero};
use thyllore_math_core::{one_minus_smootherstep_quadratic_poly, poly_symmetric_moments};

// Tapered capsule as a compact-support shell in the squared distance to the segment [a, b]:
//   density = 1 inside, faded by a smootherstep over the last `edge_width_q` of
//   delta = |p - nearest axis point|^2 - radius(lambda)^2.
// Along a ray delta is quadratic on each of the three regions (both caps and the body), so on
// every piece between knots the density is one polynomial integrated by exact power moments.
// Mirrored in shaders/include/volume_capsule.glsl.

const EMPTY_INTERVAL_EPSILON: f32 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CapsuleRegion {
    CapStart,
    Body,
    CapEnd,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CapsuleAxis {
    direction: Vector3<f32>,
    length: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VolumeCapsule {
    pub a: Vector3<f32>,
    pub b: Vector3<f32>,
    pub radius_start: f32,
    pub radius_end: f32,
    pub edge_width_q: f32,
}

fn cap_delta_quadratic(
    offset: Vector3<f32>,
    direction: Vector3<f32>,
    radius: f32,
) -> (f32, f32, f32) {
    (
        1.0,
        2.0 * offset.dot(direction),
        offset.magnitude2() - radius * radius,
    )
}

fn density_from_delta(delta: f32, edge_width_q: f32) -> f32 {
    if delta >= 0.0 {
        return 0.0;
    }
    if delta <= -edge_width_q {
        return 1.0;
    }
    let v = (delta + edge_width_q) / edge_width_q;
    1.0 - v * v * v * (10.0 - v * (15.0 - 6.0 * v))
}

impl VolumeCapsule {
    fn axis(&self) -> Option<CapsuleAxis> {
        let delta = self.b - self.a;
        let length = delta.magnitude();
        (length >= RAY_LINEAR_COEFFICIENT_EPSILON).then(|| CapsuleAxis {
            direction: delta / length,
            length,
        })
    }

    fn region_of_point(&self, point: Vector3<f32>, axis: CapsuleAxis) -> CapsuleRegion {
        let lambda = (point - self.a).dot(axis.direction);
        if lambda <= 0.0 {
            CapsuleRegion::CapStart
        } else if lambda >= axis.length {
            CapsuleRegion::CapEnd
        } else {
            CapsuleRegion::Body
        }
    }

    /// `delta(s) = |p(s) - nearest axis point|^2 - radius^2` of one region, as coefficients
    /// `(s^2, s, 1)` of the ray parameter; the direction must be normalised.
    fn delta_quadratic(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        region: CapsuleRegion,
        axis: CapsuleAxis,
    ) -> (f32, f32, f32) {
        match region {
            CapsuleRegion::CapStart => {
                cap_delta_quadratic(origin - self.a, direction, self.radius_start)
            }
            CapsuleRegion::CapEnd => {
                cap_delta_quadratic(origin - self.b, direction, self.radius_end)
            }
            CapsuleRegion::Body => {
                let offset = origin - self.a;
                let lambda_0 = offset.dot(axis.direction);
                let lambda_1 = direction.dot(axis.direction);
                let taper = (self.radius_end - self.radius_start) / axis.length;
                let radius_0 = self.radius_start + taper * lambda_0;
                let radius_1 = taper * lambda_1;
                (
                    1.0 - lambda_1 * lambda_1 - radius_1 * radius_1,
                    2.0 * (offset.dot(direction) - lambda_0 * lambda_1 - radius_0 * radius_1),
                    offset.magnitude2() - lambda_0 * lambda_0 - radius_0 * radius_0,
                )
            }
        }
    }

    fn delta_at_point(&self, point: Vector3<f32>, axis: CapsuleAxis) -> f32 {
        let region = self.region_of_point(point, axis);
        self.delta_quadratic(point, Vector3::zero(), region, axis).2
    }

    pub fn density_at(&self, point: Vector3<f32>) -> f32 {
        let Some(axis) = self.axis() else {
            return 0.0;
        };
        density_from_delta(self.delta_at_point(point, axis), self.edge_width_q)
    }

    /// Region boundaries and support boundaries of the ray, appended unsorted.
    pub fn collect_knots(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        t_near: f32,
        t_far: f32,
        knots: &mut RayKnots,
    ) {
        let Some(axis) = self.axis() else {
            return;
        };

        let lambda_0 = (origin - self.a).dot(axis.direction);
        let lambda_1 = direction.dot(axis.direction);
        if lambda_1.abs() >= RAY_LINEAR_COEFFICIENT_EPSILON {
            knots.push(-lambda_0 / lambda_1, t_near, t_far);
            knots.push((axis.length - lambda_0) / lambda_1, t_near, t_far);
        }

        for region in [
            CapsuleRegion::CapStart,
            CapsuleRegion::Body,
            CapsuleRegion::CapEnd,
        ] {
            let (delta_2, delta_1, delta_0) = self.delta_quadratic(origin, direction, region, axis);
            for boundary in [0.0, -self.edge_width_q] {
                knots.push_quadratic_roots(delta_2, delta_1, delta_0 - boundary, t_near, t_far);
            }
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

    /// Integral of the density over the piece [s0, s1], which must not cross a knot.
    pub fn piece_integral(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        s0: f32,
        s1: f32,
    ) -> f32 {
        let length = s1 - s0;
        if length <= EMPTY_INTERVAL_EPSILON {
            return 0.0;
        }
        let Some(axis) = self.axis() else {
            return 0.0;
        };

        let s_mid = s0 + 0.5 * length;
        let mid = origin + direction * s_mid;
        let delta_mid = self.delta_at_point(mid, axis);
        if delta_mid >= 0.0 {
            return 0.0;
        }
        if delta_mid <= -self.edge_width_q {
            return length;
        }

        let region = self.region_of_point(mid, axis);
        let (delta_2, delta_1, delta_0) = self.delta_quadratic(origin, direction, region, axis);
        let inv_width = 1.0 / self.edge_width_q;
        let edge = one_minus_smootherstep_quadratic_poly(
            (delta_2 * s_mid * s_mid + delta_1 * s_mid + delta_0 + self.edge_width_q) * inv_width,
            (2.0 * delta_2 * s_mid + delta_1) * length * inv_width,
            delta_2 * length * length * inv_width,
        );
        (length * poly_symmetric_moments(&edge)).max(0.0)
    }

    pub fn ray_emission(
        &self,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        t_near: f32,
        t_far: f32,
    ) -> f32 {
        if t_far <= t_near {
            return 0.0;
        }

        let center_offset = ((self.a + self.b) * 0.5 - origin).dot(direction);
        let centered_origin = origin + direction * center_offset;
        let (centered_near, centered_far) = (t_near - center_offset, t_far - center_offset);

        self.knots(centered_origin, direction, centered_near, centered_far)
            .pieces()
            .map(|(s0, s1)| self.piece_integral(centered_origin, direction, s0, s1))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn midpoint_reference_steps(
        capsule: &VolumeCapsule,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        t_near: f32,
        t_far: f32,
        steps: usize,
    ) -> f64 {
        let ds = (t_far - t_near) as f64 / steps as f64;
        (0..steps)
            .map(|i| {
                let s = t_near as f64 + (i as f64 + 0.5) * ds;
                let point = origin + direction * s as f32;
                capsule.density_at(point) as f64 * ds
            })
            .sum()
    }

    fn assert_matches_reference(
        capsule: &VolumeCapsule,
        origin: Vector3<f32>,
        direction: Vector3<f32>,
        bounds: (f32, f32),
    ) {
        let (t_near, t_far) = bounds;
        let direction = direction.normalize();
        let expected = midpoint_reference_steps(capsule, origin, direction, t_near, t_far, 1 << 17);
        let actual = capsule.ray_emission(origin, direction, t_near, t_far) as f64;
        let tolerance = 1e-3 * expected.max(1e-3);
        assert!(
            (actual - expected).abs() <= tolerance,
            "origin {:?} dir {:?} actual {actual}, expected {expected}",
            origin,
            direction
        );
    }

    fn tapered_capsule() -> VolumeCapsule {
        VolumeCapsule {
            a: Vector3::new(-1.0, 0.0, 0.0),
            b: Vector3::new(1.5, 0.5, 0.0),
            radius_start: 0.4,
            radius_end: 0.15,
            edge_width_q: 0.09,
        }
    }

    #[test]
    fn parallel_ray_matches_midpoint_quadrature() {
        let capsule = tapered_capsule();
        let axis = (capsule.b - capsule.a).normalize();
        let origin = capsule.a - axis * 2.0;
        assert_matches_reference(&capsule, origin, axis, (0.0, 8.0));
        assert_matches_reference(&capsule, capsule.b + axis * 2.0, -axis, (0.0, 8.0));
    }

    #[test]
    fn perpendicular_ray_matches_midpoint_quadrature() {
        let capsule = tapered_capsule();
        let origin = Vector3::new(0.0, 0.1, -3.0);
        assert_matches_reference(&capsule, origin, Vector3::new(0.0, 0.0, 1.0), (0.0, 6.0));
    }

    #[test]
    fn tapered_and_constant_radius_match_midpoint_quadrature() {
        let tapered = tapered_capsule();
        let constant = VolumeCapsule {
            radius_end: tapered.radius_start,
            ..tapered
        };
        let origin = Vector3::new(-2.0, 1.3, -1.7);
        let direction = Vector3::new(0.7, -0.55, 0.9);
        assert_matches_reference(&tapered, origin, direction, (0.0, 6.0));
        assert_matches_reference(&constant, origin, direction, (0.0, 6.0));
    }

    #[test]
    fn missing_ray_integrates_to_zero() {
        let capsule = tapered_capsule();
        let origin = Vector3::new(-3.0, 4.0, 0.0);
        let direction = Vector3::new(1.0, 0.0, 0.0).normalize();
        assert_eq!(capsule.ray_emission(origin, direction, 0.0, 8.0), 0.0);
    }

    fn thin_tapered_capsule() -> VolumeCapsule {
        VolumeCapsule {
            a: Vector3::new(0.0, 0.0, 0.0),
            b: Vector3::new(0.0, -1.0, 0.0),
            radius_start: 0.1,
            radius_end: 0.05,
            edge_width_q: 0.005,
        }
    }

    fn start_cap_diagonal_rays() -> [(Vector3<f32>, Vector3<f32>); 11] {
        [
            (Vector3::new(0.30, -0.24, 0.0), Vector3::new(-1.0, 1.0, 0.0)),
            (Vector3::new(0.30, 0.24, 0.0), Vector3::new(-1.0, -1.0, 0.0)),
            (Vector3::new(0.30, 0.30, 0.0), Vector3::new(-1.0, -1.0, 0.0)),
            (Vector3::new(0.30, 0.02, 0.0), Vector3::new(-1.0, 0.6, 0.0)),
            (Vector3::new(0.30, -0.02, 0.0), Vector3::new(-1.0, 0.6, 0.0)),
            (
                Vector3::new(0.25, -0.25, 0.04),
                Vector3::new(-1.0, 1.0, -0.1),
            ),
            (
                Vector3::new(0.25, 0.25, -0.04),
                Vector3::new(-1.0, -1.0, 0.1),
            ),
            (
                Vector3::new(0.30, -0.15, 0.0),
                Vector3::new(-1.0, 0.577, 0.0),
            ),
            (
                Vector3::new(0.30, 0.15, 0.0),
                Vector3::new(-1.0, -0.577, 0.0),
            ),
            (Vector3::new(-0.30, 0.20, 0.0), Vector3::new(1.0, -1.0, 0.0)),
            (Vector3::new(-0.30, -0.20, 0.0), Vector3::new(1.0, 1.0, 0.0)),
        ]
    }

    #[test]
    fn diagonal_rays_across_start_cap_match_midpoint_quadrature() {
        let capsule = thin_tapered_capsule();
        for (origin, direction) in start_cap_diagonal_rays() {
            assert_matches_reference(&capsule, origin, direction, (0.0, 1.5));
        }
    }

    #[test]
    fn far_origin_diagonal_rays_match_midpoint_quadrature() {
        let capsule = thin_tapered_capsule();
        for (origin, direction) in start_cap_diagonal_rays() {
            let dir = direction.normalize();
            let shifted_origin = origin - dir * 14.0;
            assert_matches_reference(&capsule, shifted_origin, dir, (0.0, 17.0));
        }
    }
}
