use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::device::RRDevice;
use crate::descriptor::{
    RRAutoExposureAverageDescriptorSet, RRAutoExposureHistogramDescriptorSet,
    RRBloomDescriptorSets, RRDofDescriptorSet, RRToneMapDescriptorSet, AUTO_EXPOSURE_AVERAGE,
    AUTO_EXPOSURE_HISTOGRAM, BLOOM_DOWNSAMPLE, BLOOM_UPSAMPLE, DOF, ONION_SKIN_COMPOSITE,
    ONION_SKIN_GHOST, TONEMAP,
};
use crate::pipeline::{
    BlendConfig, DepthTestConfig, PipelineBuilder, PushConstantConfig, RRPipeline,
    VertexInputConfig,
};
use crate::render::RRRender;
use crate::renderer::push_constants::OnionSkinPushConstants;
use crate::renderer::tonemap::ToneMapPushConstants;
use crate::resource::graphics_resource::GraphicsResources;
use crate::resource::{BloomChain, OnionSkinPassResources, RayTracingData};

impl RayTracingData {
    pub unsafe fn create_onion_skin_pipeline(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrrender: &RRRender,
        graphics_resources: &GraphicsResources,
        offscreen_resolve_image_view: vk::ImageView,
        offscreen_format: vk::Format,
        width: u32,
        height: u32,
    ) -> Result<()> {
        let (ghost_image, ghost_image_memory, ghost_image_view, ghost_sampler) =
            OnionSkinPassResources::create_ghost_buffer(instance, rrdevice, width, height)?;

        let ghost_render_pass = OnionSkinPassResources::create_ghost_render_pass(rrdevice)?;

        let render_layouts = [
            &graphics_resources.frame_set.layout,
            &graphics_resources.materials.layout,
            &graphics_resources.objects.layout,
        ];

        let ghost_pipeline = PipelineBuilder::from_pass(&ONION_SKIN_GHOST)
            .vertex_input(VertexInputConfig::Standard)
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .custom_render_pass(ghost_render_pass)
            .mrt_attachments(1)
            .msaa_samples(vk::SampleCountFlags::_1)
            .depth_test(DepthTestConfig {
                test_enable: false,
                write_enable: false,
                compare_op: vk::CompareOp::ALWAYS,
            })
            .blend(BlendConfig {
                enable: true,
                src_color_factor: vk::BlendFactor::SRC_ALPHA,
                dst_color_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                color_op: vk::BlendOp::ADD,
                src_alpha_factor: vk::BlendFactor::SRC_ALPHA,
                dst_alpha_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                alpha_op: vk::BlendOp::ADD,
            })
            .descriptor_layouts(&render_layouts)
            .push_constants(PushConstantConfig {
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: std::mem::size_of::<OnionSkinPushConstants>() as u32,
            })
            .build(rrdevice, rrrender, Some(vk::Extent2D { width, height }))?;

        let ghost_framebuffer = OnionSkinPassResources::create_single_framebuffer(
            rrdevice,
            ghost_render_pass,
            ghost_image_view,
            width,
            height,
        )?;

        let composite_render_pass =
            OnionSkinPassResources::create_composite_render_pass(rrdevice, offscreen_format)?;

        let (composite_descriptor_layout, composite_descriptor_set) =
            OnionSkinPassResources::create_composite_descriptor(
                rrdevice,
                ghost_image_view,
                ghost_sampler,
            )?;

        let composite_pipeline = PipelineBuilder::from_pass(&ONION_SKIN_COMPOSITE)
            .vertex_input(VertexInputConfig::Custom {
                bindings: vec![],
                attributes: vec![],
            })
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .polygon_mode(vk::PolygonMode::FILL)
            .no_depth_test()
            .custom_render_pass(composite_render_pass)
            .msaa_samples(vk::SampleCountFlags::_1)
            .blend(BlendConfig {
                enable: true,
                src_color_factor: vk::BlendFactor::ONE,
                dst_color_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                color_op: vk::BlendOp::ADD,
                src_alpha_factor: vk::BlendFactor::ONE,
                dst_alpha_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                alpha_op: vk::BlendOp::ADD,
            })
            .descriptor_layouts(&[&composite_descriptor_layout])
            .build(rrdevice, rrrender, Some(vk::Extent2D { width, height }))?;

        let composite_framebuffer = OnionSkinPassResources::create_single_framebuffer(
            rrdevice,
            composite_render_pass,
            offscreen_resolve_image_view,
            width,
            height,
        )?;

        self.onion_skin_pass = Some(OnionSkinPassResources {
            ghost_image,
            ghost_image_memory,
            ghost_image_view,
            ghost_sampler,
            ghost_render_pass,
            ghost_framebuffer,
            ghost_pipeline,
            composite_render_pass,
            composite_framebuffer,
            composite_pipeline,
            composite_descriptor_layout,
            composite_descriptor_set,
            width,
            height,
        });

        log!("Created onion skin pass: {}x{}", width, height);
        Ok(())
    }

    pub unsafe fn create_tonemap_pipeline(
        &mut self,
        rrdevice: &RRDevice,
        rrrender: &RRRender,
        hdr_image_view: vk::ImageView,
        hdr_sampler: vk::Sampler,
        position_image_view: vk::ImageView,
        position_sampler: vk::Sampler,
        scene_buffer: vk::Buffer,
        scene_buffer_size: vk::DeviceSize,
        offscreen_render_pass: vk::RenderPass,
        offscreen_extent: vk::Extent2D,
        frames_in_flight: usize,
    ) -> Result<()> {
        let tonemap_descriptor = RRToneMapDescriptorSet::new(rrdevice, frames_in_flight)?;
        tonemap_descriptor.write_all(
            rrdevice,
            hdr_image_view,
            hdr_sampler,
            position_image_view,
            position_sampler,
            scene_buffer,
            scene_buffer_size,
        )?;

        let tonemap_pipeline = PipelineBuilder::from_pass(&TONEMAP)
            .vertex_input(VertexInputConfig::Custom {
                bindings: vec![],
                attributes: vec![],
            })
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .polygon_mode(vk::PolygonMode::FILL)
            .depth_test(DepthTestConfig {
                test_enable: true,
                write_enable: true,
                compare_op: vk::CompareOp::ALWAYS,
            })
            .custom_render_pass(offscreen_render_pass)
            .descriptor_layouts(&[&tonemap_descriptor.layout])
            .push_constants(PushConstantConfig {
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: std::mem::size_of::<ToneMapPushConstants>() as u32,
            })
            .build(rrdevice, rrrender, Some(offscreen_extent))?;

        self.tonemap_pipeline = Some(tonemap_pipeline);
        self.tonemap_descriptor = Some(tonemap_descriptor);

        Ok(())
    }

    pub unsafe fn create_bloom_pipelines(
        &mut self,
        rrdevice: &RRDevice,
        rrrender: &RRRender,
        bloom_chain: &BloomChain,
        frames_in_flight: usize,
    ) -> Result<()> {
        let bloom_descriptors =
            RRBloomDescriptorSets::new(rrdevice, bloom_chain.mip_count(), frames_in_flight)?;

        let downsample_pipeline = PipelineBuilder::from_pass(&BLOOM_DOWNSAMPLE)
            .vertex_input(VertexInputConfig::Custom {
                bindings: vec![],
                attributes: vec![],
            })
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .polygon_mode(vk::PolygonMode::FILL)
            .no_depth_test()
            .custom_render_pass(bloom_chain.downsample_render_pass)
            .msaa_samples(vk::SampleCountFlags::_1)
            .descriptor_layouts(&[&bloom_descriptors.layout])
            .push_constants(PushConstantConfig {
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: 12,
            })
            .build(rrdevice, rrrender, None)?;

        let upsample_pipeline = PipelineBuilder::from_pass(&BLOOM_UPSAMPLE)
            .vertex_input(VertexInputConfig::Custom {
                bindings: vec![],
                attributes: vec![],
            })
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .polygon_mode(vk::PolygonMode::FILL)
            .no_depth_test()
            .custom_render_pass(bloom_chain.upsample_render_pass)
            .msaa_samples(vk::SampleCountFlags::_1)
            .blend(BlendConfig {
                enable: true,
                src_color_factor: vk::BlendFactor::ONE,
                dst_color_factor: vk::BlendFactor::ONE,
                color_op: vk::BlendOp::ADD,
                src_alpha_factor: vk::BlendFactor::ONE,
                dst_alpha_factor: vk::BlendFactor::ONE,
                alpha_op: vk::BlendOp::ADD,
            })
            .descriptor_layouts(&[&bloom_descriptors.layout])
            .build(rrdevice, rrrender, None)?;

        self.bloom_downsample_pipeline = Some(downsample_pipeline);
        self.bloom_upsample_pipeline = Some(upsample_pipeline);
        self.bloom_descriptors = Some(bloom_descriptors);
        log!(
            "Created bloom pipelines with {} mip levels",
            bloom_chain.mip_count()
        );

        Ok(())
    }

    pub unsafe fn create_dof_pipeline(
        &mut self,
        rrdevice: &RRDevice,
        rrrender: &RRRender,
        hdr_image_view: vk::ImageView,
        hdr_sampler: vk::Sampler,
        depth_image_view: vk::ImageView,
        depth_sampler: vk::Sampler,
        dof_render_pass: vk::RenderPass,
    ) -> Result<()> {
        let dof_descriptor = RRDofDescriptorSet::new(rrdevice)?;
        dof_descriptor.update_image_views(
            rrdevice,
            hdr_image_view,
            hdr_sampler,
            depth_image_view,
            depth_sampler,
        )?;

        let dof_pipeline = PipelineBuilder::from_pass(&DOF)
            .vertex_input(VertexInputConfig::Custom {
                bindings: vec![],
                attributes: vec![],
            })
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .polygon_mode(vk::PolygonMode::FILL)
            .no_depth_test()
            .custom_render_pass(dof_render_pass)
            .msaa_samples(vk::SampleCountFlags::_1)
            .descriptor_layouts(&[&dof_descriptor.layout])
            .push_constants(PushConstantConfig {
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: 32,
            })
            .build(rrdevice, rrrender, None)?;

        self.dof_pipeline = Some(dof_pipeline);
        self.dof_descriptor = Some(dof_descriptor);
        log!("Created DOF pipeline and descriptor set");

        Ok(())
    }

    pub unsafe fn create_auto_exposure_pipelines(
        &mut self,
        rrdevice: &RRDevice,
        hdr_image_view: vk::ImageView,
        hdr_sampler: vk::Sampler,
        histogram_buffer: vk::Buffer,
        histogram_buffer_size: u64,
        luminance_buffer: vk::Buffer,
        luminance_buffer_size: u64,
        frames_in_flight: usize,
    ) -> Result<()> {
        let histogram_descriptor =
            RRAutoExposureHistogramDescriptorSet::new(rrdevice, frames_in_flight)?;
        histogram_descriptor.update_bindings(
            rrdevice,
            hdr_image_view,
            hdr_sampler,
            histogram_buffer,
            histogram_buffer_size,
        )?;

        let histogram_push_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .offset(0)
            .size(12)
            .build();

        let histogram_pipeline = RRPipeline::new_compute_with_push_constants(
            rrdevice,
            &AUTO_EXPOSURE_HISTOGRAM,
            &[&histogram_descriptor.layout],
            &[histogram_push_range],
        )?;

        let average_descriptor = RRAutoExposureAverageDescriptorSet::new(rrdevice)?;
        average_descriptor.update_bindings(
            rrdevice,
            histogram_buffer,
            histogram_buffer_size,
            luminance_buffer,
            luminance_buffer_size,
        )?;

        let average_push_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .offset(0)
            .size(40)
            .build();

        let average_pipeline = RRPipeline::new_compute_with_push_constants(
            rrdevice,
            &AUTO_EXPOSURE_AVERAGE,
            &[&average_descriptor.layout],
            &[average_push_range],
        )?;

        self.auto_exposure_histogram_pipeline = Some(histogram_pipeline);
        self.auto_exposure_average_pipeline = Some(average_pipeline);
        self.auto_exposure_histogram_descriptor = Some(histogram_descriptor);
        self.auto_exposure_average_descriptor = Some(average_descriptor);

        Ok(())
    }
}
