use cgmath::{InnerSpace, Vector3};

use crate::tangent_frame_around;

/// The upper median of `values`; sorts them in place.
pub fn median(values: &mut [f32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f32::total_cmp);
    Some(values[values.len() / 2])
}

pub fn centroid(points: &[Vector3<f32>]) -> Option<Vector3<f32>> {
    if points.is_empty() {
        return None;
    }
    let sum = points
        .iter()
        .fold(Vector3::new(0.0, 0.0, 0.0), |sum, p| sum + p);
    Some(sum / points.len() as f32)
}

/// The point on the plane through the origin perpendicular to the unit `axis` at the middle of the
/// points' extents across the axis; None without points.
pub fn extent_centre_across(points: &[Vector3<f32>], axis: Vector3<f32>) -> Option<Vector3<f32>> {
    let (reference, side) = tangent_frame_around(axis);
    let middle = |direction: Vector3<f32>| {
        let projected = points.iter().map(|p| p.dot(direction));
        let lo = projected.clone().reduce(f32::min)?;
        let hi = projected.reduce(f32::max)?;
        Some((lo + hi) * 0.5)
    };
    Some(reference * middle(reference)? + side * middle(side)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_is_the_upper_middle_value() {
        assert_eq!(median(&mut [3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&mut [4.0, 1.0, 3.0, 2.0]), Some(3.0));
        assert_eq!(median(&mut []), None);
    }

    #[test]
    fn extent_centre_ignores_where_the_points_are_dense() {
        let mut points = vec![Vector3::new(-1.0, 5.0, 0.0), Vector3::new(3.0, -2.0, 0.0)];
        points.extend((0..10).map(|_| Vector3::new(3.0, 1.0, 0.0)));

        let centre = extent_centre_across(&points, Vector3::unit_y()).unwrap();
        assert!((centre.x - 1.0).abs() < 1e-6 && centre.y.abs() < 1e-6 && centre.z.abs() < 1e-6);
        assert_eq!(extent_centre_across(&[], Vector3::unit_y()), None);
    }

    #[test]
    fn centroid_of_nothing_is_none() {
        assert_eq!(centroid(&[]), None);
        let points = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(2.0, 4.0, 6.0)];
        assert_eq!(centroid(&points), Some(Vector3::new(1.0, 2.0, 3.0)));
    }
}
