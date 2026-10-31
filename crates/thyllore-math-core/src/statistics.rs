use cgmath::Vector3;

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
    fn centroid_of_nothing_is_none() {
        assert_eq!(centroid(&[]), None);
        let points = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(2.0, 4.0, 6.0)];
        assert_eq!(centroid(&points), Some(Vector3::new(1.0, 2.0, 3.0)));
    }
}
