use crate::app::{App, AppData};
use rust_rendering::vulkanr::buffer::*;
use rust_rendering::vulkanr::command::*;
use rust_rendering::vulkanr::data as vulkan_data;
use rust_rendering::vulkanr::data::SceneUniformData;
use rust_rendering::vulkanr::descriptor::*;
use rust_rendering::vulkanr::device::*;
use rust_rendering::vulkanr::image::*;
use rust_rendering::vulkanr::pipeline::{
    PipelineBuilder, RRPipeline, VertexInputConfig, PushConstantConfig,
};
use rust_rendering::vulkanr::render::*;
use rust_rendering::vulkanr::vulkan::*;
use rust_rendering::vulkanr::raytracing::acceleration::*;
use rust_rendering::logger::logger::*;

use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

impl App {
    pub(crate) unsafe fn init_ray_tracing(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        log::info!("Initializing Ray Tracing resources...");

        let gbuffer = RRGBuffer::new(
            instance,
            rrdevice,
            data.rrswapchain.swapchain_extent.width,
            data.rrswapchain.swapchain_extent.height,
        )?;

        gbuffer.transition_layouts(rrdevice, data.rrcommand_pool.command_pool)?;
        data.gbuffer = Some(gbuffer);
        log::info!("Created G-Buffer");

        create_gbuffer_render_pass(instance, rrdevice, &mut data.rrrender)?;

        if let Some(ref gbuffer) = data.gbuffer {
            create_gbuffer_framebuffer(instance, rrdevice, &mut data.rrrender, gbuffer)?;
        }
        log::info!("Created G-Buffer render pass and framebuffer");

        data.gbuffer_descriptor_set = Some(RRDescriptorSet::new(rrdevice, &data.rrswapchain));

        log::info!("Ray Tracing initialization complete");
        Ok(())
    }

    pub(crate) unsafe fn build_acceleration_structures(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        log::info!("Building acceleration structures...");

        let mut acceleration_structure = RRAccelerationStructure::new();

        for rrdata in &data.model_descriptor_set.rrdata {
            let blas = RRAccelerationStructure::create_blas(
                instance,
                rrdevice,
                &data.rrcommand_pool,
                &rrdata.vertex_buffer.buffer,
                rrdata.vertex_data.vertices.len() as u32,
                std::mem::size_of::<vulkan_data::Vertex>() as u32,
                &rrdata.index_buffer.buffer,
                rrdata.vertex_data.indices.len() as u32,
            )?;

            acceleration_structure.blas_list.push(blas);
            log::info!("Created BLAS for mesh");
        }

        if !acceleration_structure.blas_list.is_empty() {
            let tlas = RRAccelerationStructure::create_tlas(
                instance,
                rrdevice,
                &data.rrcommand_pool,
                &acceleration_structure.blas_list,
            )?;
            acceleration_structure.tlas = tlas;
            log::info!("Created TLAS with {} instances", acceleration_structure.blas_list.len());
        }

        data.acceleration_structure = Some(acceleration_structure);
        log::info!("Acceleration structures built successfully");
        Ok(())
    }

    pub(crate) unsafe fn create_ray_tracing_pipelines(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        log!("Creating Ray Tracing pipelines...");

        let gbuffer = RRGBuffer::new(
            instance,
            rrdevice,
            data.rrswapchain.swapchain_extent.width,
            data.rrswapchain.swapchain_extent.height,
        )?;
        data.gbuffer = Some(gbuffer);
        log!("Created G-Buffer resources (position, normal, shadow mask images)");

        if let Some(ref gbuffer) = data.gbuffer {
            create_gbuffer_framebuffer(instance, rrdevice, &mut data.rrrender, gbuffer)?;
            log!("Created G-Buffer framebuffer");
        }

        let mut gbuffer_desc = RRDescriptorSet::new(
            rrdevice,
            &data.rrswapchain,
        );

        for rrdata in &data.model_descriptor_set.rrdata {
            gbuffer_desc.rrdata.push(rrdata.clone());
        }

        RRDescriptorSet::create_descriptor_set(rrdevice, &data.rrswapchain, &mut gbuffer_desc)?;

        let gbuffer_pipeline = PipelineBuilder::new(
            "src/shaders/gbufferVert.spv",
            "src/shaders/gbufferFrag.spv",
        )
        .vertex_input(VertexInputConfig::Standard)
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .polygon_mode(vk::PolygonMode::FILL)
        .custom_render_pass(data.rrrender.gbuffer_render_pass)
        .mrt_attachments(3)
        .msaa_samples(vk::SampleCountFlags::_1)
        .descriptor_layouts(vec![gbuffer_desc.descriptor_set_layout])
        .build(rrdevice, &data.rrrender, Some(data.rrswapchain.swapchain_extent))?;

        data.gbuffer_descriptor_set = Some(gbuffer_desc);
        data.gbuffer_pipeline = Some(gbuffer_pipeline);
        log!("Created G-Buffer descriptor set and pipeline");

        let (scene_buffer, scene_memory) = create_buffer(
            instance,
            rrdevice,
            std::mem::size_of::<SceneUniformData>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        data.scene_uniform_buffer = Some(scene_buffer);
        data.scene_uniform_buffer_memory = Some(scene_memory);

        let mut ray_query_descriptor = RRRayQueryDescriptorSet {
            descriptor_set_layout: RRRayQueryDescriptorSet::create_layout(rrdevice)?,
            descriptor_pool: RRRayQueryDescriptorSet::create_pool(rrdevice)?,
            descriptor_set: vk::DescriptorSet::null(),
        };

        if let (Some(ref gbuffer), Some(ref accel_struct)) = (&data.gbuffer, &data.acceleration_structure) {
            if let Some(tlas) = accel_struct.tlas.acceleration_structure {
                ray_query_descriptor.allocate_and_update(
                    rrdevice,
                    gbuffer.position_image_view,
                    gbuffer.normal_image_view,
                    gbuffer.shadow_mask_image_view,
                    tlas,
                    scene_buffer,
                )?;
            }
        }

        let ray_query_pipeline = RRPipeline::new_compute(
            rrdevice,
            "src/shaders/rayQueryShadow.spv",
            &[ray_query_descriptor.descriptor_set_layout],
        )?;
        data.ray_query_pipeline = Some(ray_query_pipeline);
        data.ray_query_descriptor = Some(ray_query_descriptor);
        log::info!("Created Ray Query descriptor set and pipeline");

        let gbuffer_sampler = create_texture_sampler(rrdevice, 1)?;
        data.gbuffer_sampler = Some(gbuffer_sampler);

        let mut composite_descriptor = RRCompositeDescriptorSet {
            descriptor_set_layout: RRCompositeDescriptorSet::create_layout(rrdevice)?,
            descriptor_pool: RRCompositeDescriptorSet::create_pool(rrdevice)?,
            descriptor_set: vk::DescriptorSet::null(),
        };

        if let Some(ref gbuffer) = data.gbuffer {
            composite_descriptor.allocate_and_update(
                rrdevice,
                gbuffer.position_image_view,
                gbuffer_sampler,
                gbuffer.normal_image_view,
                gbuffer_sampler,
                gbuffer.shadow_mask_image_view,
                gbuffer_sampler,
                gbuffer.albedo_image_view,
                gbuffer_sampler,
                scene_buffer,
            )?;
        }

        let composite_pipeline = PipelineBuilder::new(
            "src/shaders/compositeVert.spv",
            "src/shaders/compositeFrag.spv",
        )
        .vertex_input(VertexInputConfig::Custom {
            bindings: vec![],
            attributes: vec![],
        })
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .polygon_mode(vk::PolygonMode::FILL)
        .no_depth_test()
        .descriptor_layouts(vec![composite_descriptor.descriptor_set_layout])
        .push_constants(PushConstantConfig {
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: 4,
        })
        .build(rrdevice, &data.rrrender, Some(data.rrswapchain.swapchain_extent))?;

        data.composite_pipeline = Some(composite_pipeline);
        data.composite_descriptor = Some(composite_descriptor);
        log::info!("Created composite descriptor set and pipeline");

        log::info!("Ray Tracing pipelines created successfully");
        Ok(())
    }
}
