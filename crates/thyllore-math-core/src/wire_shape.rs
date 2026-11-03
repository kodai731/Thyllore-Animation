use std::f32::consts::TAU;

use cgmath::{InnerSpace, Vector3};

/// A line of a wireframe, from one point to the other.
pub type WireSegment = ([f32; 3], [f32; 3]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CirclePlane {
    XY,
    XZ,
    YZ,
}

const MIN_CIRCLE_SEGMENTS: u32 = 4;
const DEGENERATE_AXIS_LENGTH: f32 = 1e-6;

/// The circle of `radius` around `center` in `plane`, as `segments` lines (at least four).
pub fn wire_circle_segments(
    center: [f32; 3],
    radius: f32,
    segments: u32,
    plane: CirclePlane,
) -> Vec<WireSegment> {
    let count = segments.max(MIN_CIRCLE_SEGMENTS);
    let point_at = |index: u32| {
        let angle = index as f32 / count as f32 * TAU;
        let (sin, cos) = angle.sin_cos();
        let (u, v) = (cos * radius, sin * radius);
        match plane {
            CirclePlane::XY => [center[0] + u, center[1] + v, center[2]],
            CirclePlane::XZ => [center[0] + u, center[1], center[2] + v],
            CirclePlane::YZ => [center[0], center[1] + u, center[2] + v],
        }
    };
    (0..count)
        .map(|index| (point_at(index), point_at(index + 1)))
        .collect()
}

/// A sphere as its three axis-aligned great circles.
pub fn wire_sphere_segments(center: [f32; 3], radius: f32, segments: u32) -> Vec<WireSegment> {
    [CirclePlane::XY, CirclePlane::XZ, CirclePlane::YZ]
        .into_iter()
        .flat_map(|plane| wire_circle_segments(center, radius, segments, plane))
        .collect()
}

/// A capsule as a wire sphere at each end and four lines along the axis; a capsule without
/// length is one wire sphere.
pub fn wire_capsule_segments(
    head: [f32; 3],
    tail: [f32; 3],
    radius: f32,
    segments: u32,
) -> Vec<WireSegment> {
    let axis = Vector3::from(tail) - Vector3::from(head);
    if axis.magnitude() < DEGENERATE_AXIS_LENGTH {
        return wire_sphere_segments(head, radius, segments);
    }

    let mut lines = wire_sphere_segments(head, radius, segments);
    lines.extend(wire_sphere_segments(tail, radius, segments));

    let (right, up) = perpendicular_axes(axis.normalize());
    for offset in [right, -right, up, -up] {
        let shift = offset * radius;
        lines.push((
            (Vector3::from(head) + shift).into(),
            (Vector3::from(tail) + shift).into(),
        ));
    }
    lines
}

fn perpendicular_axes(direction: Vector3<f32>) -> (Vector3<f32>, Vector3<f32>) {
    let up_hint = if direction.y.abs() < 0.99 {
        Vector3::unit_y()
    } else {
        Vector3::unit_x()
    };
    let right = direction.cross(up_hint).normalize();
    let up = direction.cross(right).normalize();
    (right, up)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
        (Vector3::from(a) - Vector3::from(b)).magnitude()
    }

    #[test]
    fn a_sphere_is_three_circles_of_the_requested_segments() {
        let lines = wire_sphere_segments([0.0, 0.0, 0.0], 1.0, 8);
        assert_eq!(lines.len(), 24);
    }

    #[test]
    fn every_sphere_point_lies_on_the_sphere() {
        let center = [1.0, 2.0, 3.0];
        for (from, to) in wire_sphere_segments(center, 0.5, 8) {
            assert!((distance(from, center) - 0.5).abs() < 1e-5);
            assert!((distance(to, center) - 0.5).abs() < 1e-5);
        }
    }

    #[test]
    fn fewer_than_four_segments_become_four() {
        assert_eq!(wire_sphere_segments([0.0; 3], 1.0, 1).len(), 12);
    }

    #[test]
    fn a_capsule_without_length_is_one_sphere() {
        let point = [0.3, 0.2, 0.1];
        assert_eq!(
            wire_capsule_segments(point, point, 1.0, 8),
            wire_sphere_segments(point, 1.0, 8)
        );
    }

    #[test]
    fn a_capsule_adds_four_axis_lines_to_two_spheres() {
        let lines = wire_capsule_segments([0.0; 3], [2.0, 0.0, 0.0], 0.5, 8);
        assert_eq!(lines.len(), 48 + 4);
        for (from, to) in &lines[48..] {
            assert!((distance(*from, *to) - 2.0).abs() < 1e-5);
        }
    }
}
