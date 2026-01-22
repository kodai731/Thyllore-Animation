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

#[derive(Clone, Debug)]
pub struct BillboardTransform {
    pub position: Vector3<f32>,
    pub model_matrix: Matrix4<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct BillboardInfo {
    pub transform: Option<BillboardTransform>,
    pub vertices: Vec<BillboardVertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer_handle: VertexBufferHandle,
    pub index_buffer_handle: IndexBufferHandle,
}

#[derive(Clone, Debug, Default)]
pub struct BillboardRenderData {
    pub pipeline_id: Option<PipelineId>,
    pub object_index: usize,
    pub descriptor_set: RRBillboardDescriptorSet,
    pub texture: Option<RRImage>,
}

#[derive(Clone, Debug, Default)]
pub struct BillboardData {
    pub info: BillboardInfo,
    pub render: BillboardRenderData,
}

impl BillboardData {
    pub fn transform(&self) -> Option<&BillboardTransform> {
        self.info.transform.as_ref()
    }

    pub fn transform_mut(&mut self) -> &mut Option<BillboardTransform> {
        &mut self.info.transform
    }

    pub fn vertices(&self) -> &[BillboardVertex] {
        &self.info.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.info.indices
    }

    pub fn vertex_buffer_handle(&self) -> VertexBufferHandle {
        self.info.vertex_buffer_handle
    }

    pub fn index_buffer_handle(&self) -> IndexBufferHandle {
        self.info.index_buffer_handle
    }

    pub fn pipeline_id(&self) -> Option<PipelineId> {
        self.render.pipeline_id
    }

    pub fn object_index(&self) -> usize {
        self.render.object_index
    }
}
