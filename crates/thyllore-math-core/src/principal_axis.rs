use glam::{Mat3, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct PrincipalAxis {
    pub mean: Vec3,
    pub axis: Vec3,
}

fn orient_largest_component_positive(axis: Vec3) -> Vec3 {
    let abs_x = axis.x.abs();
    let abs_y = axis.y.abs();
    let abs_z = axis.z.abs();

    if abs_x >= abs_y && abs_x >= abs_z {
        if axis.x < 0.0 {
            -axis
        } else {
            axis
        }
    } else if abs_y >= abs_x && abs_y >= abs_z {
        if axis.y < 0.0 {
            -axis
        } else {
            axis
        }
    } else {
        if axis.z < 0.0 {
            -axis
        } else {
            axis
        }
    }
}

pub fn fit_principal_axis(points: &[Vec3]) -> Option<PrincipalAxis> {
    if points.len() < 2 {
        return None;
    }

    let n = points.len() as f32;
    let mean = points.iter().fold(Vec3::ZERO, |acc, p| acc + *p) / n;

    let mut cov = Mat3::ZERO;
    for p in points {
        let c = *p - mean;
        cov += Mat3::from_cols(c * c.x, c * c.y, c * c.z);
    }

    let cols = [cov.x_axis, cov.y_axis, cov.z_axis];
    let mut best = cols[0];
    for col in &cols[1..] {
        if col.length_squared() > best.length_squared() {
            best = *col;
        }
    }

    if best.length_squared() < 1e-24 {
        return None;
    }

    let mut axis = best.normalize();
    for _ in 0..64 {
        axis = (cov * axis).normalize();
    }

    let axis = orient_largest_component_positive(axis);

    Some(PrincipalAxis { mean, axis })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_axis_aligned_x() {
        let points: Vec<Vec3> = (0..10).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
        let result = fit_principal_axis(&points).unwrap();

        assert!((result.axis.x - 1.0).abs() < 1e-6);
        assert!(result.axis.y.abs() < 1e-6);
        assert!(result.axis.z.abs() < 1e-6);

        let expected_mean = Vec3::new(4.5, 0.0, 0.0);
        assert!((result.mean - expected_mean).length() < 1e-6);
    }

    #[test]
    fn test_diagonal_direction() {
        let mut points: Vec<Vec3> = (0..10)
            .map(|i| {
                let t = i as f32;
                Vec3::new(t, t, 0.0)
            })
            .collect();

        for p in &mut points {
            p.x += (p.x * 13.7 + 7.3).fract() * 0.01 - 0.005;
            p.y += (p.y * 17.3 + 3.7).fract() * 0.01 - 0.005;
        }

        let result = fit_principal_axis(&points).unwrap();

        let expected = Vec3::new(1.0, 1.0, 0.0) / 2.0_f32.sqrt();
        let diff = (result.axis - expected).length();
        assert!(diff < 1e-3, "axis difference: {}", diff);
    }

    #[test]
    fn test_single_point() {
        let points = vec![Vec3::new(1.0, 2.0, 3.0)];
        assert!(fit_principal_axis(&points).is_none());
    }

    #[test]
    fn test_identical_points() {
        let points: Vec<Vec3> = std::iter::repeat(Vec3::new(1.0, 2.0, 3.0))
            .take(5)
            .collect();
        assert!(fit_principal_axis(&points).is_none());
    }
}
