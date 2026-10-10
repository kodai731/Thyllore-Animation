#[derive(Clone, Debug)]
pub struct BatchEffectOrbit {
    pub radius: f32,
    pub period_seconds: f32,
    pub initial: Option<cgmath::Vector3<f32>>,
}
