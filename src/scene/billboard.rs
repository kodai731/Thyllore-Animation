use cgmath::{Matrix4, Vector3};
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::PipelineId;
use crate::vulkanr::descriptor::RRBillboardDescriptorSet;
use crate::vulkanr::image::RRImage;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct BillboardVertex {
    pub pos: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl BillboardVertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<BillboardVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();
        let tex_coord = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(std::mem::size_of::<[f32; 3]>() as u32)
            .build();
        [pos, tex_coord]
    }
}

#[derive(Clone, Debug, Default)]
pub struct BillboardData {
    pub pipeline_id: Option<PipelineId>,
    pub descriptor_set: RRBillboardDescriptorSet,
    pub transform: Option<BillboardTransform>,
    pub object_index: usize,
    pub vertices: Vec<BillboardVertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: Option<vk::Buffer>,
    pub vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub index_buffer: Option<vk::Buffer>,
    pub index_buffer_memory: Option<vk::DeviceMemory>,
    pub texture: Option<RRImage>,
}

#[derive(Clone, Debug)]
pub struct BillboardTransform {
    pub position: Vector3<f32>,
    pub model_matrix: Matrix4<f32>,
}

