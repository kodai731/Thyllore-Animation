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

/// Translation, rotation and scale of an affine matrix; a collapsed axis keeps its zero scale and yields the identity rotation.
pub fn decompose(m: &Mat4) -> (Vec3, Quat, Vec3) {
    let translation = m.w_axis.truncate();
    let axis_lengths = Vec3::new(
        m.x_axis.truncate().length(),
        m.y_axis.truncate().length(),
        m.z_axis.truncate().length(),
    );
    if axis_lengths.min_element() <= COLLAPSED_AXIS_LENGTH {
        return (translation, Quat::IDENTITY, axis_lengths);
    }

    let (scale, rotation, _) = m.to_scale_rotation_translation();
    (translation, rotation, scale)
}

const COLLAPSED_AXIS_LENGTH: f32 = 1e-6;

/// The length of each axis of a matrix: Blender's `Matrix.to_scale()`.
pub fn axis_lengths(matrix: Mat4) -> Vec3 {
    Vec3::new(
        matrix.x_axis.truncate().length(),
        matrix.y_axis.truncate().length(),
        matrix.z_axis.truncate().length(),
    )
}

/// Singular means flat relative to the matrix's own scale: a uniformly small matrix stays invertible.
pub fn is_well_conditioned(matrix: Mat4) -> bool {
    let lengths = axis_lengths(matrix);
    let volume = lengths.x * lengths.y * lengths.z;
    matrix.determinant().abs() > f32::EPSILON * volume
}

pub fn invert(matrix: &Mat4, what: impl FnOnce() -> String) -> anyhow::Result<Mat4> {
    if !is_well_conditioned(*matrix) {
        anyhow::bail!("{} is not invertible", what());
    }
    Ok(matrix.inverse())
}

/// World points in the local space of the object whose world matrix is `world`.
pub fn world_to_local_points(world: &Mat4, points: &[[f32; 3]]) -> anyhow::Result<Vec<[f32; 3]>> {
    let to_local = invert(world, || "object world matrix".to_string())?;
    Ok(points
        .iter()
        .map(|point| to_local.transform_point3(Vec3::from(*point)).into())
        .collect())
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
    fn world_to_local_points_undo_the_object_transform() {
        let world = Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0));
        let points = world_to_local_points(&world, &[[1.5, 0.0, 0.0]]).unwrap();
        assert_eq!(points, vec![[0.5, 0.0, 0.0]]);
    }

    #[test]
    fn a_flat_matrix_is_reported_and_a_small_one_inverts() {
        let flat = Mat4::from_scale(Vec3::new(1.0, 1.0, 0.0));
        assert!(world_to_local_points(&flat, &[]).is_err());
        let tiny = Mat4::from_scale(Vec3::splat(0.001));
        let points = world_to_local_points(&tiny, &[[0.001, 0.0, 0.0]]).unwrap();
        assert!(
            Vec3::from(points[0]).abs_diff_eq(Vec3::X, 1e-4),
            "{points:?}"
        );
    }

    #[test]
    fn decompose_of_a_collapsed_axis_is_finite() {
        let flat = Mat4::from_scale(Vec3::new(1.0, 0.0, 1.0));
        let (translation, rotation, scale) = decompose(&flat);
        assert!(translation.abs_diff_eq(Vec3::ZERO, 1e-6));
        assert_eq!(rotation, Quat::IDENTITY);
        assert!(scale.abs_diff_eq(Vec3::new(1.0, 0.0, 1.0), 1e-6));
    }

    #[test]
    fn decompose_keeps_a_mirrored_axis_as_a_negative_scale() {
        let mirrored = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        let (_, rotation, scale) = decompose(&mirrored);
        assert!(rotation.is_finite());
        assert!(scale.abs_diff_eq(Vec3::new(-1.0, 1.0, 1.0), 1e-6));
    }

    #[test]
    fn test_decompose_identity() {
        let (translation, rotation, scale) = decompose(&Mat4::IDENTITY);
        assert!(translation.abs_diff_eq(Vec3::ZERO, 1e-5));
        assert!(rotation.abs_diff_eq(Quat::IDENTITY, 1e-5));
        assert!(scale.abs_diff_eq(Vec3::ONE, 1e-5));
    }
}
