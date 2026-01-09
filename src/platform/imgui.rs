use vulkanalia::prelude::v1_0::*;

#[derive(Clone, Debug, Default)]
pub struct ImguiData {
    pub pipeline: Option<vk::Pipeline>,
    pub pipeline_layout: Option<vk::PipelineLayout>,
    pub descriptor_set: Option<vk::DescriptorSet>,
    pub descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    pub descriptor_pool: Option<vk::DescriptorPool>,
    pub font_image: Option<vk::Image>,
    pub font_image_memory: Option<vk::DeviceMemory>,
    pub font_image_view: Option<vk::ImageView>,
    pub sampler: Option<vk::Sampler>,
    pub vertex_buffer: Option<vk::Buffer>,
    pub vertex_buffer_memory: Option<vk::DeviceMemory>,
    pub vertex_buffer_size: vk::DeviceSize,
    pub index_buffer: Option<vk::Buffer>,
    pub index_buffer_memory: Option<vk::DeviceMemory>,
    pub index_buffer_size: vk::DeviceSize,
}
