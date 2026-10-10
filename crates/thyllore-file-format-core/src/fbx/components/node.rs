use cgmath::{Matrix4, Quaternion};

#[derive(Clone, Debug)]
pub struct BoneNode {
    pub name: String,
    pub parent: Option<String>,
    pub local_transform: Matrix4<f32>,
    pub default_translation: [f32; 3],
    pub default_rotation: Quaternion<f32>,
    pub default_scaling: [f32; 3],
}
