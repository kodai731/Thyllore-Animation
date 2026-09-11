use anyhow::Result;
use cgmath::SquareMatrix;
use std::rc::Rc;
use thyllore_math_core::AffineRows3x4;
use vulkanalia::prelude::v1_0::*;

use crate::command::RRCommandPool;
use crate::core::device::RRDevice;
use crate::core::swapchain::RRSwapchain;
use crate::data::{self as vulkan_data, SceneUniformData};
use crate::descriptor::ReflectedSetLayout;
use crate::descriptor::{
    CompositeGBufferViews, RRAutoExposureAverageDescriptorSet,
    RRAutoExposureHistogramDescriptorSet, RRBillboardDescriptorSet, RRBloomDescriptorSets,
    RRCompositeDescriptorSet, RRDofDescriptorSet, RRFlameDescriptorSet, RRRayQueryDescriptorSet,
    RRToneMapDescriptorSet, RRWaterCausticDescriptorSet, RRWaterDescriptorSet,
    RRWaterTraceDescriptorSet, COMPOSITE, GBUFFER, RAY_QUERY_SHADOW,
};
use crate::pipeline::{
    DepthTestConfig, PipelineBuilder, PushConstantConfig, RRPipeline, RRRayTracingPipeline,
    VertexInputConfig,
};
use crate::raytracing::RRAccelerationStructure;
use crate::raytracing::{BlasGeometry, GpuPrimitive};
use crate::render::RRRender;
use crate::renderer::push_constants::GBufferPushConstants;
use crate::resource::buffer::create_buffer;
use crate::resource::graphics_resource::{GraphicsResources, MeshBuffer};
use crate::resource::image::{create_nearest_sampler, create_texture_sampler};
use crate::resource::uniform_buffer::UniformBuffer;
use crate::resource::{GpuResource, OnionSkinPassResources, RRGBuffer};
use thyllore_effect_core::{FlameUBO, WaterUBO};

pub const MAX_FLAME_INSTANCES: usize = 4;
pub const MAX_WATER_INSTANCES: usize = 4;

#[derive(Clone, Debug, Default)]
pub struct RayTracingData {
    pub command_pool: vk::CommandPool,

    pub gbuffer: Option<RRGBuffer>,
    pub gbuffer_pipeline: Option<RRPipeline>,
    pub gbuffer_sampler: Option<vk::Sampler>,
    pub object_id_sampler: Option<vk::Sampler>,

    pub acceleration_structure: Option<RRAccelerationStructure>,

    pub ray_query_pipeline: Option<RRPipeline>,
    pub ray_query_descriptor: Option<RRRayQueryDescriptorSet>,

    pub composite_pipeline: Option<RRPipeline>,
    pub composite_descriptor: Option<RRCompositeDescriptorSet>,

    pub tonemap_pipeline: Option<RRPipeline>,
    pub tonemap_descriptor: Option<RRToneMapDescriptorSet>,

    pub bloom_downsample_pipeline: Option<RRPipeline>,
    pub bloom_upsample_pipeline: Option<RRPipeline>,
    pub bloom_descriptors: Option<RRBloomDescriptorSets>,

    pub dof_pipeline: Option<RRPipeline>,
    pub dof_descriptor: Option<RRDofDescriptorSet>,

    pub auto_exposure_histogram_pipeline: Option<RRPipeline>,
    pub auto_exposure_average_pipeline: Option<RRPipeline>,
    pub auto_exposure_histogram_descriptor: Option<RRAutoExposureHistogramDescriptorSet>,
    pub auto_exposure_average_descriptor: Option<RRAutoExposureAverageDescriptorSet>,

    pub onion_skin_pass: Option<OnionSkinPassResources>,

    pub flame_shading_pipeline: Option<RRPipeline>,
    pub flame_descriptor: Option<RRFlameDescriptorSet>,
    pub flame_ubo: Option<UniformBuffer<FlameUBO>>,

    pub water_shading_pipeline: Option<RRPipeline>,
    pub water_descriptor: Option<RRWaterDescriptorSet>,
    pub water_ubo: Option<UniformBuffer<WaterUBO>>,

    pub water_trace_pipeline: Option<RRRayTracingPipeline>,
    pub water_trace_descriptor: Option<RRWaterTraceDescriptorSet>,

    pub water_caustic_splat_pipeline: Option<RRPipeline>,
    pub water_caustic_apply_pipeline: Option<RRPipeline>,
    pub water_caustic_descriptor: Option<RRWaterCausticDescriptorSet>,

    pub flame_sdf_image: vk::Image,
    pub flame_sdf_image_memory: vk::DeviceMemory,
    pub flame_sdf_image_view: vk::ImageView,
    pub flame_sdf_sampler: vk::Sampler,

    pub scene_uniform_buffer: Option<vk::Buffer>,
    pub scene_uniform_buffer_memory: Option<vk::DeviceMemory>,
}

impl RayTracingData {
    pub fn has_valid_tlas(&self) -> bool {
        self.acceleration_structure
            .as_ref()
            .and_then(|a| a.tlas.acceleration_structure)
            .is_some()
    }

    pub fn is_available(&self) -> bool {
        self.gbuffer.is_some()
            && self.gbuffer_pipeline.is_some()
            && self.ray_query_pipeline.is_some()
            && self.composite_pipeline.is_some()
    }

    pub unsafe fn destroy_all(&mut self, rrdevice: &RRDevice) {
        let device = &rrdevice.device;

        if let Some(sampler) = self.gbuffer_sampler.take() {
            device.destroy_sampler(sampler, None);
            log!("Destroyed G-Buffer sampler");
        }

        if let Some(onion_skin_pass) = self.onion_skin_pass.take() {
            onion_skin_pass.destroy(device);
        }

        if let Some(gbuffer_pipeline) = self.gbuffer_pipeline.take() {
            gbuffer_pipeline.destroy(device);
            log!("Destroyed G-Buffer pipeline");
        }

        if let Some(mut dof_descriptor) = self.dof_descriptor.take() {
            dof_descriptor.destroy(device);
        }

        if let Some(dof_pipeline) = self.dof_pipeline.take() {
            dof_pipeline.destroy(device);
        }

        if let Some(mut descriptor) = self.auto_exposure_histogram_descriptor.take() {
            descriptor.destroy(device);
        }

        if let Some(mut descriptor) = self.auto_exposure_average_descriptor.take() {
            descriptor.destroy(device);
        }

        if let Some(pipeline) = self.auto_exposure_histogram_pipeline.take() {
            pipeline.destroy(device);
        }

        if let Some(pipeline) = self.auto_exposure_average_pipeline.take() {
            pipeline.destroy(device);
        }

        if let Some(mut bloom_descriptors) = self.bloom_descriptors.take() {
            bloom_descriptors.destroy(device);
        }

        if let Some(bloom_downsample_pipeline) = self.bloom_downsample_pipeline.take() {
            bloom_downsample_pipeline.destroy(device);
        }

        if let Some(bloom_upsample_pipeline) = self.bloom_upsample_pipeline.take() {
            bloom_upsample_pipeline.destroy(device);
        }

        if let Some(mut tonemap_descriptor) = self.tonemap_descriptor.take() {
            tonemap_descriptor.destroy(device);
        }

        if let Some(tonemap_pipeline) = self.tonemap_pipeline.take() {
            tonemap_pipeline.destroy(device);
        }

        if let Some(mut composite_descriptor) = self.composite_descriptor.take() {
            composite_descriptor.destroy(device);
        }

        if let Some(composite_pipeline) = self.composite_pipeline.take() {
            composite_pipeline.destroy(device);
        }

        if let Some(mut flame_descriptor) = self.flame_descriptor.take() {
            flame_descriptor.destroy(device);
        }

        if let Some(flame_shading_pipeline) = self.flame_shading_pipeline.take() {
            flame_shading_pipeline.destroy(device);
        }

        if let Some(mut flame_ubo) = self.flame_ubo.take() {
            flame_ubo.destroy(device);
            log!("Destroyed flame uniform buffer");
        }

        if let Some(mut water_descriptor) = self.water_descriptor.take() {
            water_descriptor.destroy(device);
        }

        if let Some(water_shading_pipeline) = self.water_shading_pipeline.take() {
            water_shading_pipeline.destroy(device);
        }

        if let Some(mut water_ubo) = self.water_ubo.take() {
            water_ubo.destroy(device);
            log!("Destroyed water uniform buffer");
        }

        if let Some(mut water_trace_descriptor) = self.water_trace_descriptor.take() {
            water_trace_descriptor.destroy(device);
        }

        if let Some(water_trace_pipeline) = self.water_trace_pipeline.take() {
            water_trace_pipeline.destroy(device);
        }

        if let Some(mut water_caustic_descriptor) = self.water_caustic_descriptor.take() {
            water_caustic_descriptor.destroy(device);
        }

        if let Some(pipeline) = self.water_caustic_splat_pipeline.take() {
            pipeline.destroy(device);
        }

        if let Some(pipeline) = self.water_caustic_apply_pipeline.take() {
            pipeline.destroy(device);
        }

        if let (Some(buffer), Some(memory)) = (
            self.scene_uniform_buffer.take(),
            self.scene_uniform_buffer_memory.take(),
        ) {
            device.destroy_buffer(buffer, None);
            device.free_memory(memory, None);
            log!("Destroyed scene uniform buffer");
        }

        if let Some(mut ray_query_descriptor) = self.ray_query_descriptor.take() {
            ray_query_descriptor.destroy(device);
            log!("Destroyed ray query descriptor set");
        }

        if let Some(ray_query_pipeline) = self.ray_query_pipeline.take() {
            ray_query_pipeline.destroy(device);
            log!("Destroyed ray query pipeline");
        }

        if let Some(mut acceleration_structure) = self.acceleration_structure.take() {
            acceleration_structure.destroy(device);
            log!("Destroyed acceleration structure");
        }

        if let Some(mut gbuffer) = self.gbuffer.take() {
            gbuffer.destroy(rrdevice);
            log!("Destroyed G-Buffer");
        }
    }

    pub unsafe fn init_gbuffer(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
        rrcommand_pool: &RRCommandPool,
    ) -> Result<()> {
        log!("init_gbuffer: starting...");
        log!(
            "init_gbuffer: swapchain extent {}x{}",
            rrswapchain.swapchain_extent.width,
            rrswapchain.swapchain_extent.height
        );

        let gbuffer = RRGBuffer::new(
            instance,
            rrdevice,
            rrswapchain.swapchain_extent.width,
            rrswapchain.swapchain_extent.height,
        )?;

        log!("init_gbuffer: RRGBuffer::new succeeded");

        if let Err(e) = gbuffer.transition_layouts(rrdevice, rrcommand_pool.command_pool) {
            log_warn!("init_gbuffer: transition_layouts failed (ignored): {:?}", e);
        }

        self.gbuffer = Some(gbuffer);

        log!(
            "init_gbuffer: completed, gbuffer is_some: {}",
            self.gbuffer.is_some()
        );
        Ok(())
    }

    pub unsafe fn build_acceleration_structures(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &Rc<RRCommandPool>,
        meshes: &[MeshBuffer],
        mesh_transforms: &[cgmath::Matrix4<f32>],
        procedurals: &[GpuPrimitive],
    ) -> Result<()> {
        log!("Building acceleration structures...");

        let mut acceleration_structure = RRAccelerationStructure::new();

        // Collect vertex_buffers in the same order as BLAS creation
        let vertex_buffers: Vec<_> = meshes
            .iter()
            .filter(|mesh| mesh.render_to_gbuffer)
            .map(|mesh| {
                (
                    &mesh.vertex_buffer.buffer,
                    mesh.vertex_data.vertices.len() as u32,
                    std::mem::size_of::<vulkan_data::Vertex>() as u32,
                    &mesh.index_buffer.buffer,
                    mesh.vertex_data.indices.len() as u32,
                )
            })
            .collect();

        for (mesh_index, mesh) in meshes.iter().enumerate() {
            if !mesh.render_to_gbuffer {
                continue;
            }

            let mut blas = RRAccelerationStructure::create_blas(
                instance,
                rrdevice,
                rrcommand_pool,
                &mesh.vertex_buffer.buffer,
                mesh.vertex_data.vertices.len() as u32,
                std::mem::size_of::<vulkan_data::Vertex>() as u32,
                &mesh.index_buffer.buffer,
                mesh.vertex_data.indices.len() as u32,
            )?;

            let model = mesh_transforms
                .get(mesh_index)
                .copied()
                .unwrap_or_else(cgmath::Matrix4::identity);
            blas.transform = vk::TransformMatrixKHR {
                matrix: AffineRows3x4::from_mat4(model).rows,
            };

            acceleration_structure.blas_list.push(blas);
            log!("Created BLAS for mesh");
        }

        let mut hit_table_entries: Vec<(cgmath::Matrix4<f32>, [f32; 4])> = Vec::new();

        for primitive in procedurals {
            if let BlasGeometry::ProceduralAabb { aabb } = &primitive.geometry {
                let blas = RRAccelerationStructure::create_procedural_blas(
                    instance,
                    rrdevice,
                    rrcommand_pool,
                    &primitive.model,
                    *aabb,
                )?;
                acceleration_structure.procedural_blas.push(blas);
            }
            hit_table_entries.push((primitive.model, primitive.params));
        }

        let tlas = RRAccelerationStructure::create_tlas(
            instance,
            rrdevice,
            rrcommand_pool,
            &acceleration_structure.blas_list,
            &acceleration_structure.procedural_blas,
        )?;
        acceleration_structure.tlas = tlas;
        log!(
            "Created TLAS with {} mesh + {} procedural instances",
            acceleration_structure.blas_list.len(),
            acceleration_structure.procedural_blas.len()
        );

        acceleration_structure.fill_hit_shading_table(
            instance,
            rrdevice,
            &vertex_buffers,
            &hit_table_entries,
        )?;

        self.acceleration_structure = Some(acceleration_structure);
        log!("Acceleration structures built successfully");
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
        hdr_render_pass: Option<vk::RenderPass>,
    ) -> Result<()> {
        let render_layouts = [
            &graphics_resources.frame_set.layout,
            &graphics_resources.materials.layout,
            &graphics_resources.objects.layout,
        ];

        self.gbuffer_pipeline = Some(build_gbuffer_pipeline(
            rrdevice,
            rrrender,
            rrswapchain,
            &render_layouts,
        )?);

        let scene_buffer = self.init_scene_uniform_buffer(instance, rrdevice)?;

        let (ray_query_descriptor, ray_query_pipeline) = build_ray_query_pipeline(
            rrdevice,
            &self.gbuffer,
            &self.acceleration_structure,
            scene_buffer,
        )?;
        self.ray_query_pipeline = Some(ray_query_pipeline);
        self.ray_query_descriptor = Some(ray_query_descriptor);

        let gbuffer_sampler = create_texture_sampler(rrdevice, 1)?;
        self.gbuffer_sampler = Some(gbuffer_sampler);

        let object_id_sampler = create_nearest_sampler(rrdevice)?;
        self.object_id_sampler = Some(object_id_sampler);

        let (composite_descriptor, composite_pipeline) = build_composite_pipeline(
            instance,
            rrdevice,
            rrswapchain,
            rrrender,
            &self.gbuffer,
            gbuffer_sampler,
            object_id_sampler,
            scene_buffer,
            billboard_descriptor_set,
            offscreen_render_pass,
            offscreen_extent,
            hdr_render_pass,
        )?;
        self.composite_pipeline = Some(composite_pipeline);
        self.composite_descriptor = Some(composite_descriptor);

        Ok(())
    }

    /// Point the ray query descriptor at the current TLAS, allocating the set
    /// on first use (the pipeline may be built before any model exists).
    pub unsafe fn bind_ray_query_tlas(&mut self, rrdevice: &RRDevice) -> Result<()> {
        let Some(tlas) = self
            .acceleration_structure
            .as_ref()
            .and_then(|accel| accel.tlas.acceleration_structure)
        else {
            return Ok(());
        };
        let (Some(descriptor), Some(gbuffer), Some(scene_buffer)) = (
            self.ray_query_descriptor.as_mut(),
            self.gbuffer.as_ref(),
            self.scene_uniform_buffer,
        ) else {
            return Ok(());
        };
        let hit_shading_table_buffer = self
            .acceleration_structure
            .as_ref()
            .and_then(|accel| accel.hit_shading_table.as_ref())
            .map(|table| table.buffer)
            .unwrap_or_else(vk::Buffer::null);

        if descriptor.descriptor_set == vk::DescriptorSet::null() {
            descriptor.allocate_and_update(
                rrdevice,
                gbuffer.position_image_view,
                gbuffer.normal_image_view,
                gbuffer.shadow_mask_image_view,
                tlas,
                scene_buffer,
                hit_shading_table_buffer,
            )?;
        } else {
            descriptor.update_tlas(rrdevice, tlas, hit_shading_table_buffer)?;
        }

        if let Some(caustic_descriptor) = self.water_caustic_descriptor.as_mut() {
            caustic_descriptor.update_tlas(rrdevice, tlas)?;
        }

        Ok(())
    }

    unsafe fn init_scene_uniform_buffer(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
    ) -> Result<vk::Buffer> {
        let (scene_buffer, scene_memory) = create_buffer(
            instance,
            rrdevice,
            std::mem::size_of::<SceneUniformData>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        self.scene_uniform_buffer = Some(scene_buffer);
        self.scene_uniform_buffer_memory = Some(scene_memory);
        Ok(scene_buffer)
    }
}

impl GpuResource for RayTracingData {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy_all(rrdevice);
    }

    fn resource_name(&self) -> &'static str {
        "RayTracingData"
    }
}

unsafe fn build_gbuffer_pipeline(
    rrdevice: &RRDevice,
    rrrender: &RRRender,
    rrswapchain: &RRSwapchain,
    render_layouts: &[&ReflectedSetLayout],
) -> Result<RRPipeline> {
    PipelineBuilder::from_pass(&GBUFFER)
        .vertex_input(VertexInputConfig::Standard)
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .polygon_mode(vk::PolygonMode::FILL)
        .custom_render_pass(rrrender.gbuffer_render_pass)
        .mrt_attachments(4)
        .no_blend_attachment(3)
        .msaa_samples(vk::SampleCountFlags::_1)
        .descriptor_layouts(render_layouts)
        .push_constants(PushConstantConfig {
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<GBufferPushConstants>() as u32,
        })
        .build(rrdevice, rrrender, Some(rrswapchain.swapchain_extent))
}

unsafe fn build_ray_query_pipeline(
    rrdevice: &RRDevice,
    gbuffer: &Option<RRGBuffer>,
    acceleration_structure: &Option<RRAccelerationStructure>,
    scene_buffer: vk::Buffer,
) -> Result<(RRRayQueryDescriptorSet, RRPipeline)> {
    let mut descriptor = RRRayQueryDescriptorSet::new(rrdevice)?;

    if let (Some(gbuffer), Some(accel_struct)) = (gbuffer, acceleration_structure) {
        if let Some(tlas) = accel_struct.tlas.acceleration_structure {
            let hit_shading_table_buffer = accel_struct
                .hit_shading_table
                .as_ref()
                .map(|t| t.buffer)
                .unwrap_or(vk::Buffer::null());
            descriptor.allocate_and_update(
                rrdevice,
                gbuffer.position_image_view,
                gbuffer.normal_image_view,
                gbuffer.shadow_mask_image_view,
                tlas,
                scene_buffer,
                hit_shading_table_buffer,
            )?;
        }
    }

    let push_constant_range = vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::COMPUTE)
        .offset(0)
        .size(std::mem::size_of::<f32>() as u32)
        .build();

    let pipeline = RRPipeline::new_compute_with_push_constants(
        rrdevice,
        &RAY_QUERY_SHADOW,
        &[&descriptor.layout],
        &[push_constant_range],
    )?;

    Ok((descriptor, pipeline))
}

#[allow(clippy::too_many_arguments)]
unsafe fn build_composite_pipeline(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrswapchain: &RRSwapchain,
    rrrender: &RRRender,
    gbuffer: &Option<RRGBuffer>,
    gbuffer_sampler: vk::Sampler,
    object_id_sampler: vk::Sampler,
    scene_buffer: vk::Buffer,
    billboard_descriptor_set: &mut RRBillboardDescriptorSet,
    offscreen_render_pass: Option<vk::RenderPass>,
    offscreen_extent: Option<vk::Extent2D>,
    hdr_render_pass: Option<vk::RenderPass>,
) -> Result<(RRCompositeDescriptorSet, RRPipeline)> {
    let mut descriptor = RRCompositeDescriptorSet::new(rrdevice)?;

    if let Some(gbuffer) = gbuffer {
        descriptor.allocate_and_update(
            instance,
            rrdevice,
            CompositeGBufferViews {
                position_image_view: gbuffer.position_image_view,
                position_sampler: gbuffer_sampler,
                normal_image_view: gbuffer.normal_image_view,
                normal_sampler: gbuffer_sampler,
                shadow_mask_image_view: gbuffer.shadow_mask_image_view,
                shadow_mask_sampler: gbuffer_sampler,
                albedo_image_view: gbuffer.albedo_image_view,
                albedo_sampler: gbuffer_sampler,
                object_id_image_view: gbuffer.object_id_image_view,
                object_id_sampler,
            },
            scene_buffer,
        )?;

        billboard_descriptor_set.update_position_sampler(
            rrdevice,
            rrswapchain,
            gbuffer.position_image_view,
            gbuffer_sampler,
        )?;
    }

    let mut builder = PipelineBuilder::from_pass(&COMPOSITE)
        .vertex_input(VertexInputConfig::Custom {
            bindings: vec![],
            attributes: vec![],
        })
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .polygon_mode(vk::PolygonMode::FILL)
        .descriptor_layouts(&[&descriptor.layout])
        .push_constants(PushConstantConfig {
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: 4,
        });

    if let Some(render_pass) = hdr_render_pass {
        builder = builder
            .no_depth_test()
            .custom_render_pass(render_pass)
            .msaa_samples(vk::SampleCountFlags::_1);
    } else if let Some(render_pass) = offscreen_render_pass {
        builder = builder
            .depth_test(DepthTestConfig {
                test_enable: true,
                write_enable: true,
                compare_op: vk::CompareOp::ALWAYS,
            })
            .custom_render_pass(render_pass);
    } else {
        builder = builder.depth_test(DepthTestConfig {
            test_enable: true,
            write_enable: true,
            compare_op: vk::CompareOp::ALWAYS,
        });
    }

    let extent = offscreen_extent.unwrap_or(rrswapchain.swapchain_extent);
    let pipeline = builder.build(rrdevice, rrrender, Some(extent))?;

    Ok((descriptor, pipeline))
}
