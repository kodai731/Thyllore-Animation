use glam::Vec3;

const DEGENERATE_SEGMENT_LENGTH_SQUARED: f32 = 1e-10;

/// The parameter in [0, 1] and the point of the segment `a`-`b` closest to `point`.
pub fn project_point_to_segment(point: Vec3, a: Vec3, b: Vec3) -> (f32, Vec3) {
    let ab = b - a;
    let length_squared = ab.length_squared();
    if length_squared < DEGENERATE_SEGMENT_LENGTH_SQUARED {
        return (0.0, a);
    }
    let t = ((point - a).dot(ab) / length_squared).clamp(0.0, 1.0);
    (t, a + ab * t)
}

pub fn distance_to_segment(point: Vec3, a: Vec3, b: Vec3) -> f32 {
    let (_, projected) = project_point_to_segment(point, a, b);
    (point - projected).length()
}

/// The shortest distance between the segments `a0`-`a1` and `b0`-`b1` (Ericson, Real-Time Collision Detection 5.1.9).
pub fn distance_between_segments(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> f32 {
    let d1 = a1 - a0;
    let d2 = b1 - b0;
    let r = a0 - b0;
    let a = d1.length_squared();
    let e = d2.length_squared();
    let f = d2.dot(r);
    if a < DEGENERATE_SEGMENT_LENGTH_SQUARED {
        return distance_to_segment(a0, b0, b1);
    }
    if e < DEGENERATE_SEGMENT_LENGTH_SQUARED {
        return distance_to_segment(b0, a0, a1);
    }

    let c = d1.dot(r);
    let b = d1.dot(d2);
    let denom = a * e - b * b;
    let mut s = if denom > 0.0 {
        ((b * f - c * e) / denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let t_unclamped = (b * s + f) / e;
    let t = t_unclamped.clamp(0.0, 1.0);
    if t != t_unclamped {
        s = ((t * b - c) / a).clamp(0.0, 1.0);
    }
    ((a0 + d1 * s) - (b0 + d2 * t)).length()
}

/// `count` points spaced evenly along the polyline's arc length, starting at its first point.
pub fn resample_polyline(points: &[Vec3], count: usize) -> Vec<Vec3> {
    let segment_lengths: Vec<f32> = points
        .windows(2)
        .map(|segment| (segment[1] - segment[0]).length())
        .collect();
    let total_length: f32 = segment_lengths.iter().sum();

    (0..count)
        .map(|index| {
            let target_arc = total_length * index as f32 / count as f32;
            point_at_arc(points, &segment_lengths, target_arc)
        })
        .collect()
}

fn point_at_arc(points: &[Vec3], segment_lengths: &[f32], arc: f32) -> Vec3 {
    let mut remaining = arc;
    for (segment, &length) in points.windows(2).zip(segment_lengths) {
        if remaining <= length && length > 0.0 {
            return segment[0] + (segment[1] - segment[0]) * (remaining / length);
        }
        remaining -= length;
    }
    points.last().copied().unwrap_or(Vec3::new(0.0, 0.0, 0.0))
}

// Ericson, Real-Time Collision Detection 5.1.9.
pub fn closest_segment_parameters(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> (f32, f32) {
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);

    if a < DEGENERATE_SEGMENT_LENGTH_SQUARED && e < DEGENERATE_SEGMENT_LENGTH_SQUARED {
        return (0.0, 0.0);
    }
    if a < DEGENERATE_SEGMENT_LENGTH_SQUARED {
        return (0.0, (f / e).clamp(0.0, 1.0));
    }

    let c = d1.dot(r);
    if e < DEGENERATE_SEGMENT_LENGTH_SQUARED {
        return ((-c / a).clamp(0.0, 1.0), 0.0);
    }

    let b = d1.dot(d2);
    let denom = a * e - b * b;
    let s = if denom > 0.0 {
        ((b * f - c * e) / denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let t = (b * s + f) / e;

    if t < 0.0 {
        ((-c / a).clamp(0.0, 1.0), 0.0)
    } else if t > 1.0 {
        (((b - c) / a).clamp(0.0, 1.0), 1.0)
    } else {
        (s, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_clamps_to_the_segment_ends() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 0.0, 0.0);
        assert_eq!(
            project_point_to_segment(Vec3::new(-1.0, 1.0, 0.0), a, b).0,
            0.0
        );
        assert_eq!(
            project_point_to_segment(Vec3::new(5.0, 1.0, 0.0), a, b).0,
            1.0
        );
        assert_eq!(
            project_point_to_segment(Vec3::new(1.0, 1.0, 0.0), a, b).0,
            0.5
        );
    }

    #[test]
    fn degenerate_segment_measures_to_its_point() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        assert!((distance_to_segment(Vec3::new(1.0, 3.0, 0.0), a, a) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn resampling_keeps_the_first_point_and_even_spacing() {
        let points = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        let resampled = resample_polyline(&points, 4);
        assert_eq!(resampled[0], points[0]);
        assert!((resampled[1] - Vec3::new(0.5, 0.0, 0.0)).length() < 1e-6);
        assert!((resampled[2] - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-6);
        assert!((resampled[3] - Vec3::new(1.0, 0.5, 0.0)).length() < 1e-6);
    }

    #[test]
    fn closest_segment_parameters_non_intersecting_interior() {
        let p1 = Vec3::new(0.0, 0.0, 0.0);
        let q1 = Vec3::new(2.0, 0.0, 0.0);
        let p2 = Vec3::new(1.0, -1.0, 1.0);
        let q2 = Vec3::new(1.0, 1.0, 1.0);
        let (s, t) = closest_segment_parameters(p1, q1, p2, q2);

        assert!(s >= 0.0 && s <= 1.0);
        assert!(t >= 0.0 && t <= 1.0);

        assert!((s - 0.5).abs() < 1e-6);
        assert!((t - 0.5).abs() < 1e-6);
    }

    #[test]
    fn closest_segment_parameters_clamped_to_endpoint() {
        let p1 = Vec3::new(0.0, 0.0, 0.0);
        let q1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(5.0, 0.0, 0.0);
        let q2 = Vec3::new(6.0, 0.0, 0.0);
        let (s, t) = closest_segment_parameters(p1, q1, p2, q2);

        assert!((s - 1.0).abs() < 1e-6);
        assert!(t.abs() < 1e-6);
    }

    #[test]
    fn closest_segment_parameters_both_degenerate() {
        let p = Vec3::new(1.0, 2.0, 3.0);
        let (s, t) = closest_segment_parameters(p, p, p, p);

        assert!(s.abs() < 1e-6);
        assert!(t.abs() < 1e-6);
    }

    #[test]
    fn distance_between_segments_covers_crossing_parallel_and_offset_cases() {
        let x = |a: f32, b: f32| (Vec3::new(a, 0.0, 0.0), Vec3::new(b, 0.0, 0.0));
        let (a0, a1) = x(-1.0, 1.0);

        let crossing =
            distance_between_segments(a0, a1, Vec3::new(0.0, -1.0, 0.5), Vec3::new(0.0, 1.0, 0.5));
        assert!((crossing - 0.5).abs() < 1e-6);

        let parallel =
            distance_between_segments(a0, a1, Vec3::new(-2.0, 0.3, 0.0), Vec3::new(2.0, 0.3, 0.0));
        assert!((parallel - 0.3).abs() < 1e-6);

        let beyond_end =
            distance_between_segments(a0, a1, Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 0.0));
        assert!((beyond_end - 1.0).abs() < 1e-6);

        let point =
            distance_between_segments(a0, a1, Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, 2.0, 0.0));
        assert!((point - 2.0).abs() < 1e-6);
    }
}
