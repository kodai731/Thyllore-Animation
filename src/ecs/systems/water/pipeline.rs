use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use super::{RRWaterCausticDescriptorSet, RRWaterDescriptorSet, WaterPushConstants};
use crate::ecs::resource::{WaterGpuState, WaterRenderTargets};
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{WATER_CAUSTIC_APPLY, WATER_CAUSTIC_SPLAT, WATER_RESOLVE};
use crate::vulkanr::pipeline::{
    BlendConfig, DepthTestConfig, PipelineBuilder, PushConstantConfig, RRPipeline,
    VertexInputConfig,
};
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{GraphicsResources, Placement, RayTracingData, UniformBuffer};
use thyllore_effect_core::{WaterUBO, WATER_MAX_INSTANCES};

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
    raytracing: &RayTracingData,
    water_targets: &WaterRenderTargets,
    hdr_color_view: vk::ImageView,
    frames_in_flight: usize,
) -> Result<WaterGpuState> {
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
        .custom_render_pass(water_targets.history.render_pass)
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
        .build(rrdevice, rrrender, Some(water_targets.extent()))?;

    let mut gpu_state = WaterGpuState {
        shading_pipeline: Some(water_shading_pipeline),
        descriptor: Some(water_descriptor),
        ubo: Some(water_ubo),
        ..Default::default()
    };

    create_water_caustic_pipelines(
        rrdevice,
        raytracing,
        &mut gpu_state,
        water_targets,
        hdr_color_view,
    )?;

    log!("Created water pipelines");
    Ok(gpu_state)
}

pub unsafe fn water_trace_blocks(
    rrdevice: &RRDevice,
    gpu_state: &WaterGpuState,
) -> Result<crate::ecs::resource::WaterTraceBlocks> {
    let Some(water_ubo) = gpu_state.ubo.as_ref() else {
        return Ok(crate::ecs::resource::WaterTraceBlocks::default());
    };
    let slot_addresses = (0..WATER_MAX_INSTANCES)
        .map(|slot| water_ubo.slot_address(&rrdevice.device, slot))
        .collect::<Result<Vec<_>>>()?;
    Ok(crate::ecs::resource::WaterTraceBlocks::new(slot_addresses))
}

/// Caustic splat/apply need the water UBO and the water buffer, so they are built
/// once the water pipeline has produced them; the TLAS is bound later if missing.
unsafe fn create_water_caustic_pipelines(
    rrdevice: &RRDevice,
    raytracing: &RayTracingData,
    gpu_state: &mut WaterGpuState,
    water_targets: &WaterRenderTargets,
    hdr_color_view: vk::ImageView,
) -> Result<()> {
    let (Some(gbuffer), Some(scene_buffer), Some(water_ubo)) = (
        raytracing.gbuffer.as_ref(),
        raytracing.scene_uniform_buffer_handle(),
        gpu_state.ubo.as_ref(),
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
        water_targets.caustic_accum.view,
        gbuffer.position_image_view,
        tlas,
        scene_buffer,
        water_ubo.handle(),
        hdr_color_view,
    )?;

    let splat_pipeline =
        RRPipeline::new_compute(rrdevice, &WATER_CAUSTIC_SPLAT, &[&descriptor.splat_layout])?;
    let apply_pipeline =
        RRPipeline::new_compute(rrdevice, &WATER_CAUSTIC_APPLY, &[&descriptor.apply_layout])?;

    gpu_state.caustic_splat_pipeline = Some(splat_pipeline);
    gpu_state.caustic_apply_pipeline = Some(apply_pipeline);
    gpu_state.caustic_descriptor = Some(descriptor);
    gpu_state.caustic_bound_tlas = tlas.unwrap_or_default();

    log!("Created water caustic pipelines");
    Ok(())
}
