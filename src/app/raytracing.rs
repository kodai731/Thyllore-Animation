use anyhow::Result;
use std::rc::Rc;
use vulkanalia::prelude::v1_0::*;

use crate::app::graphics_resource::{MeshBuffer, GraphicsResources};
use crate::vulkanr::buffer::create_buffer;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::data::{self as vulkan_data, SceneUniformData};
use crate::vulkanr::descriptor::{
    RRRayQueryDescriptorSet, RRCompositeDescriptorSet, RRBillboardDescriptorSet
};
use crate::vulkanr::image::{create_texture_sampler, create_nearest_sampler};
use crate::vulkanr::pipeline::{PipelineBuilder, RRPipeline, VertexInputConfig, PushConstantConfig};
use crate::vulkanr::raytracing::acceleration::RRAccelerationStructure;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::RRGBuffer;
use crate::vulkanr::swapchain::RRSwapchain;

#[derive(Clone, Debug, Default)]
pub struct RayTracingData {
    pub gbuffer: Option<RRGBuffer>,
    pub gbuffer_pipeline: Option<RRPipeline>,
    pub gbuffer_sampler: Option<vk::Sampler>,
    pub object_id_sampler: Option<vk::Sampler>,

    pub acceleration_structure: Option<RRAccelerationStructure>,

    pub ray_query_pipeline: Option<RRPipeline>,
    pub ray_query_descriptor: Option<RRRayQueryDescriptorSet>,

    pub composite_pipeline: Option<RRPipeline>,
    pub composite_descriptor: Option<RRCompositeDescriptorSet>,

    pub scene_uniform_buffer: Option<vk::Buffer>,
    pub scene_uniform_buffer_memory: Option<vk::DeviceMemory>,
}

impl RayTracingData {
    pub fn is_available(&self) -> bool {
        let accel = self.acceleration_structure.is_some();
        let gbuf = self.gbuffer.is_some();
        let gbuf_pipe = self.gbuffer_pipeline.is_some();
        let ray_query = self.ray_query_pipeline.is_some();
        let composite = self.composite_pipeline.is_some();

        crate::log!(
            "RayTracingData::is_available - accel:{}, gbuffer:{}, gbuffer_pipe:{}, ray_query:{}, composite:{}",
            accel, gbuf, gbuf_pipe, ray_query, composite
        );

        accel && gbuf && gbuf_pipe && ray_query && composite
    }

    pub unsafe fn init_gbuffer(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
        rrcommand_pool: &RRCommandPool,
    ) -> Result<()> {
        crate::log!("init_gbuffer: starting...");
        crate::log!("init_gbuffer: swapchain extent {}x{}",
            rrswapchain.swapchain_extent.width,
            rrswapchain.swapchain_extent.height);

        let gbuffer = RRGBuffer::new(
            instance,
            rrdevice,
            rrswapchain.swapchain_extent.width,
            rrswapchain.swapchain_extent.height,
        )?;

        crate::log!("init_gbuffer: RRGBuffer::new succeeded");

        if let Err(e) = gbuffer.transition_layouts(rrdevice, rrcommand_pool.command_pool) {
            crate::log!("init_gbuffer: transition_layouts failed (ignored): {:?}", e);
        }

        self.gbuffer = Some(gbuffer);

        crate::log!("init_gbuffer: completed, gbuffer is_some: {}", self.gbuffer.is_some());
        Ok(())
    }

    pub unsafe fn build_acceleration_structures(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &Rc<RRCommandPool>,
        meshes: &[MeshBuffer],
    ) -> Result<()> {
        log::info!("Building acceleration structures...");

        let mut acceleration_structure = RRAccelerationStructure::new();

        for mesh in meshes {
            let blas = RRAccelerationStructure::create_blas(
                instance,
                rrdevice,
                rrcommand_pool,
                &mesh.vertex_buffer.buffer,
                mesh.vertex_data.vertices.len() as u32,
                std::mem::size_of::<vulkan_data::Vertex>() as u32,
                &mesh.index_buffer.buffer,
                mesh.vertex_data.indices.len() as u32,
            )?;

            acceleration_structure.blas_list.push(blas);
            log::info!("Created BLAS for mesh");
        }

        if !acceleration_structure.blas_list.is_empty() {
            let tlas = RRAccelerationStructure::create_tlas(
                instance,
                rrdevice,
                rrcommand_pool,
                &acceleration_structure.blas_list,
            )?;
            acceleration_structure.tlas = tlas;
            log::info!("Created TLAS with {} instances", acceleration_structure.blas_list.len());
        }

        self.acceleration_structure = Some(acceleration_structure);
        log::info!("Acceleration structures built successfully");
        Ok(())
    }

    pub unsafe fn create_pipelines(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
        rrrender: &RRRender,
        graphics_resources: &GraphicsResources,
        billboard_descriptor_set: &mut RRBillboardDescriptorSet,
        offscreen_render_pass: Option<vk::RenderPass>,
        offscreen_extent: Option<vk::Extent2D>,
    ) -> Result<()> {
        crate::log!("create_pipelines: starting...");
        crate::log!("create_pipelines: gbuffer is_some: {}", self.gbuffer.is_some());
        crate::log!("create_pipelines: acceleration_structure is_some: {}", self.acceleration_structure.is_some());

        let render_layouts = [
            graphics_resources.frame_set.layout,
            graphics_resources.materials.layout,
            graphics_resources.objects.layout,
        ];

        let push_constant_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(std::mem::size_of::<u32>() as u32)
            .build();

        let gbuffer_pipeline = PipelineBuilder::new(
            "assets/shaders/gbufferVert.spv",
            "assets/shaders/gbufferFrag.spv",
        )
        .vertex_input(VertexInputConfig::Standard)
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .polygon_mode(vk::PolygonMode::FILL)
        .custom_render_pass(rrrender.gbuffer_render_pass)
        .mrt_attachments(4)
        .no_blend_attachment(3)
        .msaa_samples(vk::SampleCountFlags::_1)
        .descriptor_layouts(render_layouts.to_vec())
        .push_constants(PushConstantConfig {
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<u32>() as u32,
        })
        .build(rrdevice, rrrender, Some(rrswapchain.swapchain_extent))?;

        self.gbuffer_pipeline = Some(gbuffer_pipeline);
        log::info!("Created G-Buffer descriptor set and pipeline");

        let (scene_buffer, scene_memory) = create_buffer(
            instance,
            rrdevice,
            std::mem::size_of::<SceneUniformData>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        self.scene_uniform_buffer = Some(scene_buffer);
        self.scene_uniform_buffer_memory = Some(scene_memory);

        let mut ray_query_descriptor = RRRayQueryDescriptorSet {
            descriptor_set_layout: RRRayQueryDescriptorSet::create_layout(rrdevice)?,
            descriptor_pool: RRRayQueryDescriptorSet::create_pool(rrdevice)?,
            descriptor_set: vk::DescriptorSet::null(),
        };

        if let (Some(ref gbuffer), Some(ref accel_struct)) = (&self.gbuffer, &self.acceleration_structure) {
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

        let push_constant_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .offset(0)
            .size(std::mem::size_of::<f32>() as u32)
            .build();

        let ray_query_pipeline = RRPipeline::new_compute_with_push_constants(
            rrdevice,
            "assets/shaders/rayQueryShadow.spv",
            &[ray_query_descriptor.descriptor_set_layout],
            &[push_constant_range],
        )?;
        self.ray_query_pipeline = Some(ray_query_pipeline);
        self.ray_query_descriptor = Some(ray_query_descriptor);
        log::info!("Created Ray Query descriptor set and pipeline");

        let gbuffer_sampler = create_texture_sampler(rrdevice, 1)?;
        self.gbuffer_sampler = Some(gbuffer_sampler);

        let object_id_sampler = create_nearest_sampler(rrdevice)?;
        self.object_id_sampler = Some(object_id_sampler);

        let mut composite_descriptor = RRCompositeDescriptorSet {
            descriptor_set_layout: RRCompositeDescriptorSet::create_layout(rrdevice)?,
            descriptor_pool: RRCompositeDescriptorSet::create_pool(rrdevice)?,
            descriptor_set: vk::DescriptorSet::null(),
            selection_buffer: vk::Buffer::null(),
            selection_buffer_memory: vk::DeviceMemory::null(),
        };

        if let Some(ref gbuffer) = self.gbuffer {
            composite_descriptor.allocate_and_update(
                instance,
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
                gbuffer.object_id_image_view,
                object_id_sampler,
            )?;

            billboard_descriptor_set.update_position_sampler(
                rrdevice,
                rrswapchain,
                gbuffer.position_image_view,
                gbuffer_sampler,
            )?;
            log::info!("Updated billboard descriptor set with G-Buffer position sampler");
        }

        let mut composite_builder = PipelineBuilder::new(
            "assets/shaders/compositeVert.spv",
            "assets/shaders/compositeFrag.spv",
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
        });

        if let Some(render_pass) = offscreen_render_pass {
            composite_builder = composite_builder.custom_render_pass(render_pass);
            crate::log!("create_pipelines: using offscreen render pass for composite pipeline");
        }

        let extent = offscreen_extent.unwrap_or(rrswapchain.swapchain_extent);
        let composite_pipeline = composite_builder.build(rrdevice, rrrender, Some(extent))?;

        self.composite_pipeline = Some(composite_pipeline);
        self.composite_descriptor = Some(composite_descriptor);
        log::info!("Created composite descriptor set and pipeline");

        log::info!("Ray Tracing pipelines created successfully");
        Ok(())
    }
}
