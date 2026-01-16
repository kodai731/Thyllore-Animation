use cgmath::{Matrix4, Vector3};
use vulkanalia::vk;

use crate::vulkanr::pipeline::RRPipeline;

#[derive(Clone, Copy, Debug)]
pub struct CameraState {
    pub position: Vector3<f32>,
    pub direction: Vector3<f32>,
}

pub struct UpdateContext {
    pub time: f32,
    pub delta_time: f32,
}

pub struct RenderContext<'a> {
    pub camera: &'a CameraState,
    pub image_index: usize,
}

pub trait Updatable {
    fn update(&mut self, ctx: &UpdateContext);
}

pub trait Renderable {
    fn object_index(&self) -> usize;
    fn model_matrix(&self, ctx: &RenderContext) -> Matrix4<f32>;
    fn pipeline(&self) -> &RRPipeline;
    fn vertex_buffer(&self) -> vk::Buffer;
    fn index_buffer(&self) -> vk::Buffer;
    fn index_count(&self) -> u32;
}
