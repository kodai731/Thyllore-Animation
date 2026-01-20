use cgmath::{Matrix4, Vector3};

use crate::ecs::resource::PipelineId;
use crate::render::{IndexBufferHandle, VertexBufferHandle};
use crate::vulkanr::descriptor::RRBillboardDescriptorSet;
use crate::vulkanr::image::RRImage;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct BillboardVertex {
    pub pos: [f32; 3],
    pub tex_coord: [f32; 2],
}

#[derive(Clone, Debug, Default)]
pub struct BillboardData {
    pub pipeline_id: Option<PipelineId>,
    pub descriptor_set: RRBillboardDescriptorSet,
    pub transform: Option<BillboardTransform>,
    pub object_index: usize,
    pub vertices: Vec<BillboardVertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer_handle: VertexBufferHandle,
    pub index_buffer_handle: IndexBufferHandle,
    pub texture: Option<RRImage>,
}

#[derive(Clone, Debug)]
pub struct BillboardTransform {
    pub position: Vector3<f32>,
    pub model_matrix: Matrix4<f32>,
}

