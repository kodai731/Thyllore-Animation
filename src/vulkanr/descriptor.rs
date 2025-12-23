use super::data::*;
use super::device::*;
use super::swapchain::*;
use super::vulkan::*;

#[derive(Clone, Debug, Default)]
pub struct RRDescriptorSet {
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub rrdata: Vec<RRData>,
}

impl RRDescriptorSet {
    pub unsafe fn new(rrdevice: &RRDevice, rrswapchain: &RRSwapchain) -> Self {
        let mut rrdescriptor_sets = RRDescriptorSet::default();
        let _ = create_descriptor_set_layout(rrdevice, &mut rrdescriptor_sets);
        let _ = create_descriptor_pool(rrdevice, rrswapchain, &mut rrdescriptor_sets);
        println!("rrdescriptor_sets: {:?}", rrdescriptor_sets);
        rrdescriptor_sets
    }

    pub unsafe fn create_descriptor_set(
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
        rrdescriptor_set: &mut RRDescriptorSet,
    ) -> Result<()> {
        if let Err(e) = create_descriptor_sets(rrdevice, rrswapchain, rrdescriptor_set) {
            println!("error creating descriptor set: {:?}", e);
        }
        Ok(())
    }

    pub unsafe fn delete_data(&mut self, rrdevice: &RRDevice) {
        // Free allocated descriptor sets before deleting data
        if !self.descriptor_sets.is_empty() {
            rrdevice.device.free_descriptor_sets(
                self.descriptor_pool,
                &self.descriptor_sets,
            ).ok(); // Ignore errors if pool was already reset
            self.descriptor_sets.clear();
        }

        // Delete rrdata resources
        for i in 0..self.rrdata.len() {
            self.rrdata[i].delete(rrdevice);
        }
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        if !self.descriptor_sets.is_empty() {
            device.free_descriptor_sets(
                self.descriptor_pool,
                &self.descriptor_sets,
            ).ok();
            self.descriptor_sets.clear();
        }

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
unsafe fn create_descriptor_set_layout(
    rrdevice: &RRDevice,
    rrdescriptor_set: &mut RRDescriptorSet,
) -> Result<()> {
    // The descriptor layout specifies the types of resources that are going to be accessed by the pipeline,
    // just like a render pass specifies the types of attachments that will be accessed
    let ubo_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX);

    let sampler_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let bindings = &[ubo_binding, sampler_binding];
    let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);
    rrdescriptor_set.descriptor_set_layout =
        rrdevice.device.create_descriptor_set_layout(&info, None)?;

    Ok(())
}

unsafe fn create_descriptor_pool(
    rrdevice: &RRDevice,
    rrswapchain: &RRSwapchain,
    rrdescriptor_set: &mut RRDescriptorSet,
) -> Result<()> {
    // Support up to 30 meshes (30 meshes * swapchain_images)
    // This allows models with many sub-meshes to be loaded dynamically
    let max_meshes = 30;
    let descriptor_count = (rrswapchain.swapchain_images.len() * max_meshes) as u32;

    let ubo_size = vk::DescriptorPoolSize::builder()
        .type_(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(descriptor_count);

    let sampler_size = vk::DescriptorPoolSize::builder()
        .type_(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(descriptor_count);

    let pool_sizes = &[ubo_size, sampler_size];
    let info = vk::DescriptorPoolCreateInfo::builder()
        .pool_sizes(pool_sizes)
        .max_sets(descriptor_count)
        .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET); // Allow individual descriptor set freeing
    rrdescriptor_set.descriptor_pool = rrdevice.device.create_descriptor_pool(&info, None)?;

    Ok(())
}

unsafe fn create_descriptor_sets(
    rrdevice: &RRDevice,
    rrswapchain: &RRSwapchain,
    rrdescriptor_set: &mut RRDescriptorSet,
) -> Result<()> {
    /*
    A descriptor is a way for shaders to freely access resources like buffers and images
    Usage of descriptors consists of three parts:

    Specify a descriptor layout during pipeline creation
    Allocate a descriptor set from a descriptor pool
    Bind the descriptor set during rendering
     */
    // Create descriptor sets for each rrdata and each swapchain image
    // Total descriptor sets = rrdata.len() * swapchain_images.len()
    let num_sets = rrswapchain.swapchain_images.len() * rrdescriptor_set.rrdata.len().max(1);
    let layouts = vec![rrdescriptor_set.descriptor_set_layout; num_sets];
    println!("{}, {}", "layouts length", layouts.len());
    let info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(rrdescriptor_set.descriptor_pool)
        .set_layouts(&layouts);
    rrdescriptor_set.descriptor_sets = rrdevice.device.allocate_descriptor_sets(&info)?;

    // Update descriptor sets for each rrdata
    for j in 0..rrdescriptor_set.rrdata.len() {
        let rrdata = &rrdescriptor_set.rrdata[j];

        for i in 0..rrswapchain.swapchain_images.len() {
            // Calculate descriptor set index: j * swapchain_images.len() + i
            let descriptor_set_index = j * rrswapchain.swapchain_images.len() + i;

            let info = vk::DescriptorBufferInfo::builder()
                .buffer(rrdata.rruniform_buffers[i].buffer)
                .offset(0)
                .range(size_of::<UniformBufferObject>() as u64);
            // The configuration of descriptors is updated using the update_descriptor_sets function,
            // which takes an array of vk::WriteDescriptorSet structs as parameter.
            let buffer_info = &[info];

            let ubo_write = vk::WriteDescriptorSet::builder()
                .dst_set(rrdescriptor_set.descriptor_sets[descriptor_set_index])
                .dst_binding(0)
                .dst_array_element(0) // Remember that descriptors can be arrays, so we also need to specify the first index in the array that we want to update
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(buffer_info);

            if rrdata.image_view != vk::ImageView::null() && rrdata.sampler != vk::Sampler::null() {
                let info = vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(rrdata.image_view)
                    .sampler(rrdata.sampler);
                let image_info = &[info];

                let sampler_write = vk::WriteDescriptorSet::builder()
                    .dst_set(rrdescriptor_set.descriptor_sets[descriptor_set_index])
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(image_info);

                rrdevice.device.update_descriptor_sets(
                    &[ubo_write, sampler_write],
                    &[] as &[vk::CopyDescriptorSet],
                );
            } else {
                rrdevice
                    .device
                    .update_descriptor_sets(&[ubo_write], &[] as &[vk::CopyDescriptorSet]);
            }
        }
    }

    Ok(())
}

/// Descriptor set for Ray Query compute shader
#[derive(Clone, Debug, Default)]
pub struct RRRayQueryDescriptorSet {
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set: vk::DescriptorSet,
}

impl RRRayQueryDescriptorSet {
    /// Create Ray Query descriptor set layout
    pub unsafe fn create_layout(rrdevice: &RRDevice) -> Result<vk::DescriptorSetLayout> {
        // Binding 0: Position image (storage image, read)
        let position_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .build();

        // Binding 1: Normal image (storage image, read)
        let normal_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .build();

        // Binding 2: Shadow mask image (storage image, write)
        let shadow_mask_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(2)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .build();

        // Binding 3: Top-level acceleration structure
        let tlas_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(3)
            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .build();

        // Binding 4: Scene uniform buffer (light position, etc.)
        let scene_ubo_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(4)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .build();

        let bindings = [
            position_binding,
            normal_binding,
            shadow_mask_binding,
            tlas_binding,
            scene_ubo_binding,
        ];

        let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

        let layout = rrdevice.device.create_descriptor_set_layout(&info, None)?;
        log::info!("Created Ray Query descriptor set layout");

        Ok(layout)
    }

    /// Create descriptor pool for Ray Query
    pub unsafe fn create_pool(rrdevice: &RRDevice) -> Result<vk::DescriptorPool> {
        let storage_image_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(3); // position, normal, shadow mask

        let accel_struct_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .descriptor_count(1);

        let ubo_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1);

        let pool_sizes = [storage_image_size, accel_struct_size, ubo_size];

        let info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(1);

        let pool = rrdevice.device.create_descriptor_pool(&info, None)?;
        log::info!("Created Ray Query descriptor pool");

        Ok(pool)
    }

    /// Allocate and update descriptor set with G-Buffer images, TLAS, and scene uniform buffer
    pub unsafe fn allocate_and_update(
        &mut self,
        rrdevice: &RRDevice,
        position_image_view: vk::ImageView,
        normal_image_view: vk::ImageView,
        shadow_mask_image_view: vk::ImageView,
        tlas: vk::AccelerationStructureKHR,
        scene_uniform_buffer: vk::Buffer,
    ) -> Result<()> {
        // Allocate descriptor set
        let layouts = [self.descriptor_set_layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = rrdevice.device.allocate_descriptor_sets(&alloc_info)?;
        self.descriptor_set = descriptor_sets[0];

        // Binding 0: Position image (storage image, read)
        let position_image_info = vk::DescriptorImageInfo::builder()
            .image_view(position_image_view)
            .image_layout(vk::ImageLayout::GENERAL)
            .build();

        let position_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(std::slice::from_ref(&position_image_info))
            .build();

        // Binding 1: Normal image (storage image, read)
        let normal_image_info = vk::DescriptorImageInfo::builder()
            .image_view(normal_image_view)
            .image_layout(vk::ImageLayout::GENERAL)
            .build();

        let normal_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.descriptor_set)
            .dst_binding(1)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(std::slice::from_ref(&normal_image_info))
            .build();

        // Binding 2: Shadow mask image (storage image, write)
        let shadow_mask_info = vk::DescriptorImageInfo::builder()
            .image_view(shadow_mask_image_view)
            .image_layout(vk::ImageLayout::GENERAL)
            .build();

        let shadow_mask_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.descriptor_set)
            .dst_binding(2)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(std::slice::from_ref(&shadow_mask_info))
            .build();

        // Binding 3: TLAS (acceleration structure)
        let tlas_info = vk::WriteDescriptorSetAccelerationStructureKHR::builder()
            .acceleration_structures(std::slice::from_ref(&tlas))
            .build();

        let mut tlas_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.descriptor_set)
            .dst_binding(3)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .push_next(&mut tlas_info.clone())
            .build();
        // Note: descriptor_count is set to 1 by default

        // Binding 4: Scene uniform buffer
        let scene_buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(scene_uniform_buffer)
            .offset(0)
            .range(std::mem::size_of::<super::data::SceneUniformData>() as u64)
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
            tlas_write,
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

/// Descriptor set for Composite fragment shader
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

        // Binding 3: Scene uniform buffer (light position, color, etc.)
        let scene_ubo_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(3)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        let bindings = [
            position_binding,
            normal_binding,
            shadow_mask_binding,
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
            .descriptor_count(3); // position, normal, shadow mask

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
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
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
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
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

        // Binding 3: Scene uniform buffer
        let scene_buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(scene_uniform_buffer)
            .offset(0)
            .range(std::mem::size_of::<super::data::SceneUniformData>() as u64)
            .build();

        let scene_ubo_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.descriptor_set)
            .dst_binding(3)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(std::slice::from_ref(&scene_buffer_info))
            .build();

        // Update descriptor sets
        let writes = [
            position_write,
            normal_write,
            shadow_mask_write,
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
