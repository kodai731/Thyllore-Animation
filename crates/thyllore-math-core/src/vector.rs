pub fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn magnitude3(v: [f32; 3]) -> f32 {
    dot3(v, v).sqrt()
}

pub fn normalize3(v: [f32; 3]) -> Option<[f32; 3]> {
    let len = magnitude3(v);
    if len < 1e-6 {
        return None;
    }
    Some([v[0] / len, v[1] / len, v[2] / len])
}

pub fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn remove_parallel_component3(v: [f32; 3], parallel_to: [f32; 3]) -> [f32; 3] {
    let proj = dot3(v, parallel_to);
    [
        v[0] - proj * parallel_to[0],
        v[1] - proj * parallel_to[1],
        v[2] - proj * parallel_to[2],
    ]
}

pub fn orthonormal_basis_from_up_and_right(up: [f32; 3], right: [f32; 3]) -> Option<[[f32; 3]; 3]> {
    let up = normalize3(up)?;
    let right = remove_parallel_component3(right, up);
    let right = normalize3(right)?;
    let forward = cross3(up, right);
    Some([right, up, forward])
}

pub fn approx_equal_array3(a: &[f32; 3], b: &[f32; 3]) -> bool {
    (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5 && (a[2] - b[2]).abs() < 1e-5
}

#[cfg(test)]
mod tests {
    use super::*;

    fn almost_eq(a: [f32; 3], b: [f32; 3], eps: f32) -> bool {
        (a[0] - b[0]).abs() < eps && (a[1] - b[1]).abs() < eps && (a[2] - b[2]).abs() < eps
    }

    #[test]
    fn test_orthonormal_basis_up_y_right_x() {
        let basis = orthonormal_basis_from_up_and_right([0.0, 1.0, 0.0], [1.0, 0.0, 0.0]);
        assert!(basis.is_some());
        let [right, up, forward] = basis.unwrap();
        assert!(almost_eq(right, [1.0, 0.0, 0.0], 1e-5));
        assert!(almost_eq(up, [0.0, 1.0, 0.0], 1e-5));
        assert!(almost_eq(forward, [0.0, 0.0, -1.0], 1e-5));
    }

    #[test]
    fn test_orthonormal_basis_scaled_and_parallel_removed() {
        let basis = orthonormal_basis_from_up_and_right([0.0, 2.0, 0.0], [-1.0, 0.5, 0.0]);
        assert!(basis.is_some());
        let [right, up, forward] = basis.unwrap();
        assert!(almost_eq(right, [-1.0, 0.0, 0.0], 1e-5));
        assert!(almost_eq(up, [0.0, 1.0, 0.0], 1e-5));
        assert!(almost_eq(forward, [0.0, 0.0, 1.0], 1e-5));
    }

    #[test]
    fn test_orthonormal_basis_degenerate_parallel() {
        let basis = orthonormal_basis_from_up_and_right([0.0, 1.0, 0.0], [0.0, 2.0, 0.0]);
        assert!(basis.is_none());
    }

    #[test]
    fn test_orthonormal_basis_degenerate_zero_up() {
        let basis = orthonormal_basis_from_up_and_right([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert!(basis.is_none());
    }
}
