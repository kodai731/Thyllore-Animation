use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{
    RRWaterCausticDescriptorSet, RRWaterDescriptorSet, RRWaterTraceDescriptorSet,
    WATER_CAUSTIC_APPLY, WATER_CAUSTIC_SPLAT, WATER_RESOLVE, WATER_TRACE,
};
use crate::vulkanr::pipeline::{
    BlendConfig, DepthTestConfig, PipelineBuilder, PushConstantConfig, RRPipeline,
    RRRayTracingPipeline, VertexInputConfig,
};
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{
    GraphicsResources, HdrBuffer, Placement, RayTracingData, UniformBuffer, WaterBuffer,
};
use thyllore_effect_core::{WaterUBO, WATER_MAX_INSTANCES};
use thyllore_vulkan_core::renderer::WaterPushConstants;

fn opaque_blend() -> BlendConfig {
    BlendConfig {
        enable: false,
        src_color_factor: vk::BlendFactor::ONE,
        dst_color_factor: vk::BlendFactor::ZERO,
        color_op: vk::BlendOp::ADD,
        src_alpha_factor: vk::BlendFactor::ONE,
        dst_alpha_factor: vk::BlendFactor::ZERO,
        alpha_op: vk::BlendOp::ADD,
    }
}

pub unsafe fn create_water_pipeline(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &RRRender,
    graphics_resources: &GraphicsResources,
    raytracing: &mut RayTracingData,
    water_buffer: &WaterBuffer,
    hdr_buffer: &HdrBuffer,
    frames_in_flight: usize,
) -> Result<()> {
    let water_ubo = UniformBuffer::new(
        instance,
        rrdevice,
        WATER_MAX_INSTANCES,
        Placement::DeviceUpdated,
    )?;
    water_ubo.write_slot(rrdevice, 0, &WaterUBO::default())?;

    let water_descriptor = RRWaterDescriptorSet::new(rrdevice, frames_in_flight)?;
    let water_shading_pipeline = PipelineBuilder::from_pass(&WATER_RESOLVE)
        .vertex_input(VertexInputConfig::Custom {
            bindings: vec![],
            attributes: vec![],
        })
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .depth_test(DepthTestConfig {
            test_enable: true,
            write_enable: true,
            compare_op: vk::CompareOp::GREATER_OR_EQUAL,
        })
        .custom_render_pass(water_buffer.render_pass)
        .msaa_samples(vk::SampleCountFlags::_1)
        .mrt_attachments(2)
        .blend(opaque_blend())
        .push_constants(PushConstantConfig {
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<WaterPushConstants>() as u32,
        })
        .dynamic_states(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
        .descriptor_layouts(&[
            &graphics_resources.frame_set.layout,
            &water_descriptor.layout,
        ])
        .build(rrdevice, rrrender, Some(water_buffer.extent()))?;

    let water_trace_descriptor = RRWaterTraceDescriptorSet::new(rrdevice, frames_in_flight)?;
    let water_trace_pipeline = RRRayTracingPipeline::new(
        instance,
        rrdevice,
        &WATER_TRACE,
        &[water_trace_descriptor.layout.handle],
        &trace_push_constant_ranges(),
    )?;

    raytracing.water_shading_pipeline = Some(water_shading_pipeline);
    raytracing.water_descriptor = Some(water_descriptor);
    raytracing.water_ubo = Some(water_ubo);
    raytracing.water_trace_descriptor = Some(water_trace_descriptor);
    raytracing.water_trace_pipeline = Some(water_trace_pipeline);

    create_water_caustic_pipelines(rrdevice, raytracing, water_buffer, hdr_buffer)?;

    log!("Created water trace pipeline");
    Ok(())
}

fn trace_push_constant_ranges() -> [vk::PushConstantRange; 3] {
    let intersection_range = vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::INTERSECTION_KHR)
        .offset(0)
        .size(8)
        .build();
    let raygen_range = vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
        .offset(16)
        .size(112)
        .build();
    let closest_hit_range = vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
        .offset(96)
        .size(32)
        .build();
    [intersection_range, raygen_range, closest_hit_range]
}

/// Caustic splat/apply need the water UBO and the water buffer, so they are built
/// once the water pipeline has produced them; the TLAS is bound later if missing.
unsafe fn create_water_caustic_pipelines(
    rrdevice: &RRDevice,
    raytracing: &mut RayTracingData,
    water_buffer: &WaterBuffer,
    hdr_buffer: &HdrBuffer,
) -> Result<()> {
    let (Some(gbuffer), Some(scene_buffer), Some(water_ubo)) = (
        raytracing.gbuffer.as_ref(),
        raytracing.scene_uniform_buffer_handle(),
        raytracing.water_ubo.as_ref(),
    ) else {
        log!("Water caustic inputs are not ready, skipping caustic pipelines");
        return Ok(());
    };
    let tlas = raytracing
        .acceleration_structure
        .as_ref()
        .and_then(|accel| accel.tlas.acceleration_structure);

    let mut descriptor = RRWaterCausticDescriptorSet::new(rrdevice)?;
    descriptor.allocate_and_update(
        rrdevice,
        water_buffer.caustic_accum_view,
        gbuffer.position_image_view,
        tlas,
        scene_buffer,
        water_ubo.handle(),
        hdr_buffer.color_image_view,
    )?;

    let splat_pipeline =
        RRPipeline::new_compute(rrdevice, &WATER_CAUSTIC_SPLAT, &[&descriptor.splat_layout])?;
    let apply_pipeline =
        RRPipeline::new_compute(rrdevice, &WATER_CAUSTIC_APPLY, &[&descriptor.apply_layout])?;

    raytracing.water_caustic_splat_pipeline = Some(splat_pipeline);
    raytracing.water_caustic_apply_pipeline = Some(apply_pipeline);
    raytracing.water_caustic_descriptor = Some(descriptor);

    log!("Created water caustic pipelines");
    Ok(())
}
