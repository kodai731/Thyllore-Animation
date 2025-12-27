use crate::vulkanr::data::*;
use crate::vulkanr::core::device::*;
use crate::vulkanr::core::swapchain::*;
use crate::vulkanr::vulkan::*;

#[derive(Clone, Debug, Default)]
pub struct RRCompositeDescriptorSet {
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set: vk::DescriptorSet,
}

impl RRCompositeDescriptorSet {
    /// Create Composite descriptor set layout
    pub unsafe fn create_layout(rrdevice: &RRDevice) -> Result<vk::DescriptorSetLayout> {
        // Binding 0: Position sampler (sampled image)
        let position_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        // Binding 1: Normal sampler (sampled image)
        let normal_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        // Binding 2: Shadow mask sampler (sampled image)
        let shadow_mask_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(2)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        // Binding 3: Albedo sampler (sampled image)
        let albedo_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(3)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        // Binding 4: Scene uniform buffer (light position, color, etc.)
        let scene_ubo_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(4)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        let bindings = [
            position_binding,
            normal_binding,
            shadow_mask_binding,
            albedo_binding,
            scene_ubo_binding,
        ];

        let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

        let layout = rrdevice.device.create_descriptor_set_layout(&info, None)?;

        Ok(layout)
    }

    /// Create descriptor pool for Composite
    pub unsafe fn create_pool(rrdevice: &RRDevice) -> Result<vk::DescriptorPool> {
        let sampler_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(4); // position, normal, shadow mask, albedo

        let ubo_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1);

        let pool_sizes = [sampler_size, ubo_size];

        let info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(1);

        let pool = rrdevice.device.create_descriptor_pool(&info, None)?;

        Ok(pool)
    }

    /// Allocate and update descriptor set with G-Buffer images and scene uniform buffer
    pub unsafe fn allocate_and_update(
        &mut self,
        rrdevice: &RRDevice,
        position_image_view: vk::ImageView,
        position_sampler: vk::Sampler,
        normal_image_view: vk::ImageView,
        normal_sampler: vk::Sampler,
        shadow_mask_image_view: vk::ImageView,
        shadow_mask_sampler: vk::Sampler,
        albedo_image_view: vk::ImageView,
        albedo_sampler: vk::Sampler,
        scene_uniform_buffer: vk::Buffer,
    ) -> Result<()> {
        // Allocate descriptor set
        let layouts = [self.descriptor_set_layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = rrdevice.device.allocate_descriptor_sets(&alloc_info)?;
        self.descriptor_set = descriptor_sets[0];

        // Binding 0: Position image sampler
        let position_image_info = vk::DescriptorImageInfo::builder()
            .image_view(position_image_view)
            .sampler(position_sampler)
            .image_layout(vk::ImageLayout::GENERAL)
            .build();

        let position_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&position_image_info))
            .build();

        // Binding 1: Normal image sampler
        let normal_image_info = vk::DescriptorImageInfo::builder()
            .image_view(normal_image_view)
            .sampler(normal_sampler)
            .image_layout(vk::ImageLayout::GENERAL)
            .build();

        let normal_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.descriptor_set)
            .dst_binding(1)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&normal_image_info))
            .build();

        // Binding 2: Shadow mask image sampler
        let shadow_mask_info = vk::DescriptorImageInfo::builder()
            .image_view(shadow_mask_image_view)
            .sampler(shadow_mask_sampler)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build();

        let shadow_mask_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.descriptor_set)
            .dst_binding(2)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&shadow_mask_info))
            .build();

        // Binding 3: Albedo image sampler
        let albedo_image_info = vk::DescriptorImageInfo::builder()
            .image_view(albedo_image_view)
            .sampler(albedo_sampler)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build();

        let albedo_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.descriptor_set)
            .dst_binding(3)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&albedo_image_info))
            .build();

        // Binding 4: Scene uniform buffer
        let scene_buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(scene_uniform_buffer)
            .offset(0)
            .range(std::mem::size_of::<crate::vulkanr::data::SceneUniformData>() as u64)
            .build();

        let scene_ubo_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.descriptor_set)
            .dst_binding(4)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(std::slice::from_ref(&scene_buffer_info))
            .build();

        // Update descriptor sets
        let writes = [
            position_write,
            normal_write,
            shadow_mask_write,
            albedo_write,
            scene_ubo_write,
        ];

        rrdevice.device.update_descriptor_sets(&writes, &[] as &[vk::CopyDescriptorSet]);

        Ok(())
    }

    /// Destroy descriptor set resources
    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        if self.descriptor_pool != vk::DescriptorPool::null() {
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.descriptor_pool = vk::DescriptorPool::null();
        }

        if self.descriptor_set_layout != vk::DescriptorSetLayout::null() {
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.descriptor_set_layout = vk::DescriptorSetLayout::null();
        }
    }
}
