use cgmath::{Matrix4, Vector2};

pub struct ProjectionData {
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
    pub screen_size: Vector2<f32>,
    pub aspect: f32,
}
