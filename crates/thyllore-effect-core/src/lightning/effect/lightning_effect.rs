use super::{LightningBranching, LightningLook, LightningShape, LightningTiming};
use cgmath::{Matrix4, Quaternion, Vector3};

#[derive(Clone, Debug, PartialEq)]
pub struct LightningEffect {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub time: f32,
    pub time_scale: f32,
    pub time_offset: f32,
    pub shape: LightningShape,
    pub branch: LightningBranching,
    pub look: LightningLook,
    pub timing: LightningTiming,
}

impl Default for LightningEffect {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            time: 0.0,
            time_scale: 1.0,
            time_offset: 0.0,
            shape: LightningShape::default(),
            branch: LightningBranching::default(),
            look: LightningLook::default(),
            timing: LightningTiming::default(),
        }
    }
}

pub fn build_lightning_model_matrix(effect: &LightningEffect) -> Matrix4<f32> {
    Matrix4::from_translation(effect.position) * Matrix4::from(effect.rotation)
}
