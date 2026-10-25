use cgmath::{InnerSpace, Vector3};

const DEGENERATE_SEGMENT_LENGTH_SQUARED: f32 = 1e-10;

/// The parameter in [0, 1] and the point of the segment `a`-`b` closest to `point`.
pub fn project_point_to_segment(
    point: Vector3<f32>,
    a: Vector3<f32>,
    b: Vector3<f32>,
) -> (f32, Vector3<f32>) {
    let ab = b - a;
    let length_squared = ab.magnitude2();
    if length_squared < DEGENERATE_SEGMENT_LENGTH_SQUARED {
        return (0.0, a);
    }
    let t = ((point - a).dot(ab) / length_squared).clamp(0.0, 1.0);
    (t, a + ab * t)
}

pub fn distance_to_segment(point: Vector3<f32>, a: Vector3<f32>, b: Vector3<f32>) -> f32 {
    let (_, projected) = project_point_to_segment(point, a, b);
    (point - projected).magnitude()
}

/// `count` points spaced evenly along the polyline's arc length, starting at its first point.
pub fn resample_polyline(points: &[Vector3<f32>], count: usize) -> Vec<Vector3<f32>> {
    let segment_lengths: Vec<f32> = points
        .windows(2)
        .map(|segment| (segment[1] - segment[0]).magnitude())
        .collect();
    let total_length: f32 = segment_lengths.iter().sum();

    (0..count)
        .map(|index| {
            let target_arc = total_length * index as f32 / count as f32;
            point_at_arc(points, &segment_lengths, target_arc)
        })
        .collect()
}

fn point_at_arc(points: &[Vector3<f32>], segment_lengths: &[f32], arc: f32) -> Vector3<f32> {
    let mut remaining = arc;
    for (segment, &length) in points.windows(2).zip(segment_lengths) {
        if remaining <= length && length > 0.0 {
            return segment[0] + (segment[1] - segment[0]) * (remaining / length);
        }
        remaining -= length;
    }
    points
        .last()
        .copied()
        .unwrap_or(Vector3::new(0.0, 0.0, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_clamps_to_the_segment_ends() {
        let a = Vector3::new(0.0, 0.0, 0.0);
        let b = Vector3::new(2.0, 0.0, 0.0);
        assert_eq!(
            project_point_to_segment(Vector3::new(-1.0, 1.0, 0.0), a, b).0,
            0.0
        );
        assert_eq!(
            project_point_to_segment(Vector3::new(5.0, 1.0, 0.0), a, b).0,
            1.0
        );
        assert_eq!(
            project_point_to_segment(Vector3::new(1.0, 1.0, 0.0), a, b).0,
            0.5
        );
    }

    #[test]
    fn degenerate_segment_measures_to_its_point() {
        let a = Vector3::new(1.0, 0.0, 0.0);
        assert!((distance_to_segment(Vector3::new(1.0, 3.0, 0.0), a, a) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn resampling_keeps_the_first_point_and_even_spacing() {
        let points = [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
        ];
        let resampled = resample_polyline(&points, 4);
        assert_eq!(resampled[0], points[0]);
        assert!((resampled[1] - Vector3::new(0.5, 0.0, 0.0)).magnitude() < 1e-6);
        assert!((resampled[2] - Vector3::new(1.0, 0.0, 0.0)).magnitude() < 1e-6);
        assert!((resampled[3] - Vector3::new(1.0, 0.5, 0.0)).magnitude() < 1e-6);
    }
}
