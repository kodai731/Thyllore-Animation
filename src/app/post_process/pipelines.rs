use crate::app::{App, AppData};
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{
    RRAutoExposureAverageDescriptorSet, RRAutoExposureHistogramDescriptorSet,
    RRBloomDescriptorSets, RRDofDescriptorSet, RRToneMapDescriptorSet, AUTO_EXPOSURE_AVERAGE,
    AUTO_EXPOSURE_HISTOGRAM, BLOOM_DOWNSAMPLE, BLOOM_UPSAMPLE, DOF, TONEMAP,
};
use crate::vulkanr::pipeline::{
    BlendConfig, DepthTestConfig, PipelineBuilder, PushConstantConfig, RRPipeline,
    VertexInputConfig,
};
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::BloomChain;
use thyllore_vulkan_core::renderer::tonemap::ToneMapPushConstants;

use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

fn fullscreen_pipeline(pass: &'static crate::vulkanr::descriptor::PassShaders) -> PipelineBuilder {
    PipelineBuilder::from_pass(pass)
        .vertex_input(VertexInputConfig::Custom {
            bindings: vec![],
            attributes: vec![],
        })
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .polygon_mode(vk::PolygonMode::FILL)
}

fn additive_blend() -> BlendConfig {
    BlendConfig {
        enable: true,
        src_color_factor: vk::BlendFactor::ONE,
        dst_color_factor: vk::BlendFactor::ONE,
        color_op: vk::BlendOp::ADD,
        src_alpha_factor: vk::BlendFactor::ONE,
        dst_alpha_factor: vk::BlendFactor::ONE,
        alpha_op: vk::BlendOp::ADD,
    }
}

impl App {
    pub(crate) unsafe fn create_tonemap_pipeline_with_resources(
        rrdevice: &RRDevice,
        data: &mut AppData,
        rrrender: &RRRender,
    ) -> Result<()> {
        let (hdr_image_view, hdr_sampler) = match data.viewport.hdr_buffer {
            Some(ref hdr) => (hdr.color_image_view, hdr.sampler),
            None => {
                log!("HDR buffer not available, skipping tonemap pipeline");
                return Ok(());
            }
        };

        let (offscreen_render_pass, offscreen_extent) = match data.viewport.offscreen {
            Some(ref offscreen) => (offscreen.render_pass, offscreen.extent()),
            None => {
                log!("Offscreen not available, skipping tonemap pipeline");
                return Ok(());
            }
        };

        let position_image_view = match data.raytracing.gbuffer {
            Some(ref gbuffer) => gbuffer.position_image_view,
            None => {
                log!("GBuffer not available, skipping tonemap pipeline");
                return Ok(());
            }
        };

        let gbuffer_sampler = match data.raytracing.gbuffer_sampler {
            Some(s) => s,
            None => {
                log!("GBuffer sampler not available, skipping tonemap pipeline");
                return Ok(());
            }
        };

        let scene_buffer = match data.raytracing.scene_uniform_buffer_handle() {
            Some(b) => b,
            None => {
                log!("Scene buffer not available, skipping tonemap pipeline");
                return Ok(());
            }
        };
        let scene_buffer_size =
            std::mem::size_of::<thyllore_vulkan_core::data::SceneUniformData>() as vk::DeviceSize;

        let tonemap_descriptor =
            RRToneMapDescriptorSet::new(rrdevice, crate::app::init::MAX_FRAMES_IN_FLIGHT)?;
        tonemap_descriptor.write_all(
            rrdevice,
            hdr_image_view,
            hdr_sampler,
            position_image_view,
            gbuffer_sampler,
            scene_buffer,
            scene_buffer_size,
        )?;

        let tonemap_pipeline = fullscreen_pipeline(&TONEMAP)
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

        data.raytracing.tonemap_pipeline = Some(tonemap_pipeline);
        data.raytracing.tonemap_descriptor = Some(tonemap_descriptor);

        log!("Tonemap pipeline created successfully");
        Ok(())
    }

    pub(crate) unsafe fn create_bloom_pipelines_with_resources(
        rrdevice: &RRDevice,
        data: &mut AppData,
        rrrender: &RRRender,
    ) -> Result<()> {
        let bloom_chain = match data.viewport.bloom_chain {
            Some(ref chain) => chain,
            None => {
                log!("Bloom chain not available, skipping bloom pipelines");
                return Ok(());
            }
        };

        let (downsample_pipeline, upsample_pipeline, bloom_descriptors) =
            build_bloom_pipelines(rrdevice, rrrender, bloom_chain)?;

        data.raytracing.bloom_downsample_pipeline = Some(downsample_pipeline);
        data.raytracing.bloom_upsample_pipeline = Some(upsample_pipeline);
        data.raytracing.bloom_descriptors = Some(bloom_descriptors);

        log!("Bloom pipelines created successfully");
        Ok(())
    }

    pub(crate) unsafe fn create_dof_pipeline_with_resources(
        rrdevice: &RRDevice,
        data: &mut AppData,
        rrrender: &RRRender,
    ) -> Result<()> {
        let (hdr_image_view, hdr_sampler) = match data.viewport.hdr_buffer {
            Some(ref hdr) => (hdr.color_image_view, hdr.sampler),
            None => {
                log!("HDR buffer not available, skipping DOF pipeline");
                return Ok(());
            }
        };

        let dof_render_pass = match data.viewport.dof_buffer {
            Some(ref buf) => buf.render_pass,
            None => {
                log!("DOF buffer not available, skipping DOF pipeline");
                return Ok(());
            }
        };

        let depth_image_view = rrrender.gbuffer_depth_image_view;
        if depth_image_view == vk::ImageView::null() {
            log!("GBuffer depth image view not available, skipping DOF pipeline");
            return Ok(());
        }

        let depth_sampler = match data.raytracing.gbuffer_sampler {
            Some(s) => s,
            None => {
                log!("GBuffer sampler not available, skipping DOF pipeline");
                return Ok(());
            }
        };

        let dof_descriptor = RRDofDescriptorSet::new(rrdevice)?;
        dof_descriptor.update_image_views(
            rrdevice,
            hdr_image_view,
            hdr_sampler,
            depth_image_view,
            depth_sampler,
        )?;

        let dof_pipeline = fullscreen_pipeline(&DOF)
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

        data.raytracing.dof_pipeline = Some(dof_pipeline);
        data.raytracing.dof_descriptor = Some(dof_descriptor);

        log!("DOF pipeline created successfully");
        Ok(())
    }

    pub(crate) unsafe fn create_auto_exposure_pipelines_with_resources(
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        let (histogram_buffer, luminance_buffer) = match data.viewport.auto_exposure_buffers {
            Some(ref buf) => (buf.histogram_buffer, buf.luminance_buffer),
            None => {
                log!("AutoExposure buffers not available, skipping pipeline");
                return Ok(());
            }
        };
        let histogram_buffer_size = (256 * 4) as u64;
        let luminance_buffer_size = (2 * 4) as u64;

        let (hdr_image_view, hdr_sampler) = match data.viewport.hdr_buffer {
            Some(ref hdr) => (hdr.color_image_view, hdr.sampler),
            None => {
                log!("No HDR input available for AutoExposure");
                return Ok(());
            }
        };

        let histogram_descriptor = RRAutoExposureHistogramDescriptorSet::new(
            rrdevice,
            crate::app::init::MAX_FRAMES_IN_FLIGHT,
        )?;
        histogram_descriptor.update_bindings(
            rrdevice,
            hdr_image_view,
            hdr_sampler,
            histogram_buffer,
            histogram_buffer_size,
        )?;
        let histogram_pipeline = RRPipeline::new_compute_with_push_constants(
            rrdevice,
            &AUTO_EXPOSURE_HISTOGRAM,
            &[&histogram_descriptor.layout],
            &[compute_push_constant_range(12)],
        )?;

        let average_descriptor = RRAutoExposureAverageDescriptorSet::new(rrdevice)?;
        average_descriptor.update_bindings(
            rrdevice,
            histogram_buffer,
            histogram_buffer_size,
            luminance_buffer,
            luminance_buffer_size,
        )?;
        let average_pipeline = RRPipeline::new_compute_with_push_constants(
            rrdevice,
            &AUTO_EXPOSURE_AVERAGE,
            &[&average_descriptor.layout],
            &[compute_push_constant_range(40)],
        )?;

        data.raytracing.auto_exposure_histogram_pipeline = Some(histogram_pipeline);
        data.raytracing.auto_exposure_average_pipeline = Some(average_pipeline);
        data.raytracing.auto_exposure_histogram_descriptor = Some(histogram_descriptor);
        data.raytracing.auto_exposure_average_descriptor = Some(average_descriptor);

        log!("AutoExposure pipelines created successfully");
        Ok(())
    }
}

unsafe fn build_bloom_pipelines(
    rrdevice: &RRDevice,
    rrrender: &RRRender,
    bloom_chain: &BloomChain,
) -> Result<(RRPipeline, RRPipeline, RRBloomDescriptorSets)> {
    let bloom_descriptors = RRBloomDescriptorSets::new(
        rrdevice,
        bloom_chain.mip_count(),
        crate::app::init::MAX_FRAMES_IN_FLIGHT,
    )?;

    let downsample_pipeline = fullscreen_pipeline(&BLOOM_DOWNSAMPLE)
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

    let upsample_pipeline = fullscreen_pipeline(&BLOOM_UPSAMPLE)
        .no_depth_test()
        .custom_render_pass(bloom_chain.upsample_render_pass)
        .msaa_samples(vk::SampleCountFlags::_1)
        .blend(additive_blend())
        .descriptor_layouts(&[&bloom_descriptors.layout])
        .build(rrdevice, rrrender, None)?;

    log!(
        "Created bloom pipelines with {} mip levels",
        bloom_chain.mip_count()
    );
    Ok((downsample_pipeline, upsample_pipeline, bloom_descriptors))
}

fn compute_push_constant_range(size: u32) -> vk::PushConstantRange {
    vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::COMPUTE)
        .offset(0)
        .size(size)
        .build()
}
