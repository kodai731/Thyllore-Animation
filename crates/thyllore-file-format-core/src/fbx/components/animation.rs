use cgmath::Quaternion;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FbxAnimation {
    pub name: String,
    pub duration: f32,
    pub bone_animations: HashMap<String, BoneAnimation>,
}

#[derive(Clone, Debug)]
pub struct BoneAnimation {
    pub bone_name: String,
    pub translation_keys: Vec<KeyFrame<[f32; 3]>>,
    pub rotation_keys: Vec<KeyFrame<Quaternion<f32>>>,
    pub scale_keys: Vec<KeyFrame<[f32; 3]>>,
}

#[derive(Clone, Debug)]
pub struct KeyFrame<T> {
    pub time: f32,
    pub value: T,
}
