use glam::{Mat4, Quat};

#[derive(Clone, Debug)]
pub struct BoneNode {
    pub name: String,
    pub parent: Option<String>,
    pub local_transform: Mat4,
    pub default_translation: [f32; 3],
    pub default_rotation: Quat,
    pub default_scaling: [f32; 3],
}
