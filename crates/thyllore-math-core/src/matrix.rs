use glam::{Mat3, Mat4, Quat, Vec3};

pub fn array_from_mat4(m: Mat4) -> [[f32; 4]; 4] {
    m.to_cols_array_2d()
}

pub fn mat4_from_array(a: [[f32; 4]; 4]) -> Mat4 {
    Mat4::from_cols_array_2d(&a)
}

/// The scalar a length takes through `linear`: the mean length of its axes.
pub fn mean_axis_scale(linear: &Mat3) -> f32 {
    (linear.x_axis.length() + linear.y_axis.length() + linear.z_axis.length()) / 3.0
}

pub fn decompose(m: &Mat4) -> (Vec3, Quat, Vec3) {
    let (scale, rotation, translation) = m.to_scale_rotation_translation();
    (translation, rotation, scale)
}

/// Reads a quaternion stored as `[w, x, y, z]` from one stored as `[x, y, z, w]`.
pub fn swap(q: &Quat) -> Quat {
    Quat::from_xyzw(q.y, q.z, q.w, q.x)
}

/// The same transform between two spaces that were both scaled by `factor`: rotation kept, translation scaled.
pub fn scale_translation(m: &Mat4, factor: f32) -> Mat4 {
    let mut scaled = *m;
    scaled.w_axis.x *= factor;
    scaled.w_axis.y *= factor;
    scaled.w_axis.z *= factor;
    scaled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mat4_array_conversion() {
        let original = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let converted = mat4_from_array(array_from_mat4(original));
        assert!(original.abs_diff_eq(converted, 1e-6));
    }

    #[test]
    fn test_swap_quaternion() {
        let swapped = swap(&Quat::from_xyzw(1.0, 2.0, 3.0, 4.0));
        assert_eq!(swapped, Quat::from_xyzw(2.0, 3.0, 4.0, 1.0));
    }

    #[test]
    fn test_decompose_identity() {
        let (translation, rotation, scale) = decompose(&Mat4::IDENTITY);
        assert!(translation.abs_diff_eq(Vec3::ZERO, 1e-5));
        assert!(rotation.abs_diff_eq(Quat::IDENTITY, 1e-5));
        assert!(scale.abs_diff_eq(Vec3::ONE, 1e-5));
    }
}
