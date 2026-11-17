use glam::{Mat3, Mat4, Vec3};
use thyllore_math_core::mean_axis_scale;

use crate::humanoid::components::skeleton_input::BoneInput;

/// A bone's rest frame as the sidecar sees it: bone-local vectors to model space (engine axes, metres).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneRestFrame {
    to_model: Mat3,
    to_local: Mat3,
}

impl BoneRestFrame {
    pub fn identity_for(bone_count: usize) -> Vec<Self> {
        vec![Self::IDENTITY; bone_count]
    }

    pub const IDENTITY: Self = Self {
        to_model: Mat3::from_cols_array(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]),
        to_local: Mat3::from_cols_array(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]),
    };

    /// None when the frame is singular and cannot map model vectors back.
    pub fn new(to_model: Mat3) -> Option<Self> {
        let to_local = to_model.try_inverse()?;
        Some(Self { to_model, to_local })
    }

    /// The linear part of a rest global transform in skeleton units, scaled into metres.
    pub fn from_global_transform(global: &Mat4, metres_per_skeleton_unit: f32) -> Option<Self> {
        let linear = Mat3::from_cols(
            global.x_axis.truncate(),
            global.y_axis.truncate(),
            global.z_axis.truncate(),
        ) * metres_per_skeleton_unit;
        Self::new(linear)
    }

    /// Row-major rows of the bone-local to model-space matrix, as a sidecar consumer declares them.
    pub fn from_rows(rows: [[f32; 3]; 3]) -> Option<Self> {
        Self::new(Mat3::from_cols_array_2d(&rows).transpose())
    }

    pub fn to_model(&self, local: Vec3) -> Vec3 {
        self.to_model * local
    }

    pub fn to_local(&self, model: Vec3) -> Vec3 {
        self.to_local * model
    }

    /// Metres per bone-local unit, as the mean axis length; radii are scalars and take this.
    pub fn metres_per_local_unit(&self) -> f32 {
        mean_axis_scale(&self.to_model)
    }

    pub fn length_to_model(&self, local_length: f32) -> f32 {
        local_length * self.metres_per_local_unit()
    }

    pub fn length_to_local(&self, model_length: f32) -> f32 {
        model_length / self.metres_per_local_unit()
    }
}

/// Every bone's rest frame from its rest global transform; a singular one is an error, since colliders
/// on that bone could not be mapped back.
pub fn rest_frames_from_globals(
    rest_globals: &[Mat4],
    bones: &[BoneInput],
    metres_per_skeleton_unit: f32,
) -> anyhow::Result<Vec<BoneRestFrame>> {
    rest_globals
        .iter()
        .zip(bones)
        .map(|(global, bone)| {
            BoneRestFrame::from_global_transform(global, metres_per_skeleton_unit)
                .ok_or_else(|| anyhow::anyhow!("bone {:?} has a singular rest frame", bone.name))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotated_and_scaled_frame_round_trips_vectors_and_lengths() {
        let rotation = Mat3::from_rotation_z(f32::to_radians(90.0));
        let global = Mat4::from_mat3(rotation) * Mat4::from_scale(Vec3::splat(100.0));
        let frame = BoneRestFrame::from_global_transform(&global, 0.01).expect("regular frame");

        let model = frame.to_model(Vec3::new(1.0, 0.0, 0.0));
        assert!((model - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-6);
        let local = frame.to_local(model);
        assert!((local - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-6);
        assert!((frame.length_to_model(2.0) - 2.0).abs() < 1e-6);
        assert!((frame.length_to_local(2.0) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn rows_are_read_row_major() {
        let frame = BoneRestFrame::from_rows([[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]])
            .expect("regular frame");
        let model = frame.to_model(Vec3::new(1.0, 0.0, 0.0));
        assert!((model - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-6);
    }

    #[test]
    fn singular_frame_is_rejected() {
        assert!(BoneRestFrame::new(Mat3::from_diagonal(Vec3::splat(0.0))).is_none());
    }
}
