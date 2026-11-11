use glam::Vec3;

use crate::TAU;

/// Two unit axes orthogonal to `axis` and to each other: `(reference, side)` with `side = axis × reference`.
pub fn tangent_frame_around(axis: Vec3) -> (Vec3, Vec3) {
    let seed = if axis.x.abs() < 0.9 { Vec3::X } else { Vec3::Z };
    let reference = (seed - axis * seed.dot(axis)).normalize();
    (reference, axis.cross(reference))
}

/// `count` unit directions: `axis` itself, then a ring at `half_angle` radians from it, evenly spaced in azimuth.
pub fn cone_directions(axis: Vec3, half_angle: f32, count: usize) -> Vec<Vec3> {
    let mut directions = Vec::with_capacity(count);
    if count == 0 {
        return directions;
    }
    directions.push(axis);

    let (reference, side) = tangent_frame_around(axis);
    let ring_count = count - 1;
    for ring_index in 0..ring_count {
        let azimuth = TAU * ring_index as f32 / ring_count as f32;
        let around = reference * azimuth.cos() + side * azimuth.sin();
        directions.push((axis * half_angle.cos() + around * half_angle.sin()).normalize());
    }
    directions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_axes_are_unit_and_orthogonal() {
        for axis in [Vec3::Y, Vec3::X, Vec3::new(0.6, 0.0, 0.8)] {
            let (reference, side) = tangent_frame_around(axis);
            assert!((reference.length() - 1.0).abs() < 1e-6);
            assert!((side.length() - 1.0).abs() < 1e-6);
            assert!(reference.dot(axis).abs() < 1e-6);
            assert!(side.dot(axis).abs() < 1e-6);
            assert!(reference.dot(side).abs() < 1e-6);
        }
    }

    #[test]
    fn cone_directions_keep_the_half_angle_and_do_not_repeat() {
        let axis = Vec3::Y;
        let half_angle = 30.0_f32.to_radians();
        let directions = cone_directions(axis, half_angle, 5);

        assert_eq!(directions.len(), 5);
        assert_eq!(directions[0], axis);
        for direction in &directions[1..] {
            assert!((direction.length() - 1.0).abs() < 1e-5);
            assert!((direction.dot(axis) - half_angle.cos()).abs() < 1e-5);
        }
        for (i, a) in directions.iter().enumerate() {
            for b in &directions[i + 1..] {
                assert!((a - b).length() > 1e-3, "{a:?} repeats {b:?}");
            }
        }
        assert!(cone_directions(axis, half_angle, 0).is_empty());
        assert_eq!(cone_directions(axis, half_angle, 1), vec![axis]);
    }
}
