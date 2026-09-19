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
    RRCompositeDescriptorSet, RRDofDescriptorSet, RREffectTraceDescriptorSet,
    RRRayQueryDescriptorSet, RRToneMapDescriptorSet, COMPOSITE, GBUFFER, RAY_QUERY_SHADOW,
};
use crate::pipeline::{
    DepthTestConfig, PipelineBuilder, PushConstantConfig, RRPipeline, RRRayTracingPipeline,
    VertexInputConfig,
};
use crate::raytracing::RRAccelerationStructure;
use crate::raytracing::{BlasGeometry, GpuPrimitive};
use crate::render::RRRender;
use crate::renderer::push_constants::GBufferPushConstants;
use crate::resource::graphics_resource::{GraphicsResources, MeshBuffer};
use crate::resource::image::{create_nearest_sampler, create_texture_sampler};
use crate::resource::uniform_buffer::{Placement, UniformBuffer};
use crate::resource::{GpuResource, OnionSkinPassResources, RRGBuffer};

#[derive(Clone, Debug, Default, GpuResource)]
pub struct RayTracingData {
    #[gpu_resource(skip)]
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

    pub effect_trace_pipeline: Option<RRRayTracingPipeline>,
    pub effect_trace_descriptor: Option<RREffectTraceDescriptorSet>,

    pub scene_uniform_buffer: Option<UniformBuffer<SceneUniformData>>,
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
            procedurals,
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
        let scene_buffer_handle = self.scene_uniform_buffer_handle();
        let (Some(descriptor), Some(gbuffer), Some(scene_buffer)) = (
            self.ray_query_descriptor.as_mut(),
            self.gbuffer.as_ref(),
            scene_buffer_handle,
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

        Ok(())
    }

    unsafe fn init_scene_uniform_buffer(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
    ) -> Result<vk::Buffer> {
        let scene_uniform_buffer =
            UniformBuffer::new(instance, rrdevice, 1, Placement::HostMapped)?;
        let handle = scene_uniform_buffer.handle();
        self.scene_uniform_buffer = Some(scene_uniform_buffer);
        Ok(handle)
    }

    pub fn scene_uniform_buffer_handle(&self) -> Option<vk::Buffer> {
        self.scene_uniform_buffer
            .as_ref()
            .map(UniformBuffer::handle)
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
