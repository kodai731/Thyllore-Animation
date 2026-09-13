use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::{WindGpuState, WindRenderTargets};
use crate::ecs::systems::wind::descriptors::{
    WindResolveDescriptorSet, WindShadowBakeDescriptorSet, WindUpsampleDescriptorSet,
};
use crate::ecs::systems::wind::record::WindPushConstants;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{WIND_RESOLVE, WIND_SHADOW_BAKE, WIND_UPSAMPLE};
use crate::vulkanr::pipeline::{
    BlendConfig, PipelineBuilder, PushConstantConfig, RRPipeline, VertexInputConfig,
};
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{GraphicsResources, Placement, UniformBuffer};
use thyllore_effect_core::{WindUBO, WIND_MAX_INSTANCES};

fn premultiplied_blend() -> BlendConfig {
    BlendConfig {
        enable: true,
        src_color_factor: vk::BlendFactor::ONE,
        dst_color_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
        color_op: vk::BlendOp::ADD,
        src_alpha_factor: vk::BlendFactor::ONE,
        dst_alpha_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
        alpha_op: vk::BlendOp::ADD,
    }
}

fn fullscreen_overlay_pipeline(
    pass: &'static crate::vulkanr::descriptor::PassShaders,
    targets: &WindRenderTargets,
) -> PipelineBuilder {
    PipelineBuilder::from_pass(pass)
        .vertex_input(VertexInputConfig::Custom {
            bindings: vec![],
            attributes: vec![],
        })
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .no_depth_test()
        .custom_render_pass(targets.render_pass)
        .msaa_samples(vk::SampleCountFlags::_1)
        .blend(premultiplied_blend())
        .dynamic_states(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
}

pub unsafe fn create_wind_gpu_state(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &RRRender,
    graphics_resources: &GraphicsResources,
    targets: &WindRenderTargets,
    scene_depth_view: vk::ImageView,
) -> Result<WindGpuState> {
    let ubo = UniformBuffer::new(
        instance,
        rrdevice,
        WIND_MAX_INSTANCES,
        Placement::DeviceUpdated,
    )?;
    ubo.write_slot(rrdevice, 0, &WindUBO::default())?;

    let resolve_descriptor = WindResolveDescriptorSet::new(rrdevice)?;
    resolve_descriptor.write_all(rrdevice, &ubo, &targets.shadow_volume, scene_depth_view)?;

    let shadow_bake_descriptor = WindShadowBakeDescriptorSet::new(rrdevice)?;
    shadow_bake_descriptor.write_all(rrdevice, &ubo, &targets.shadow_volume)?;
    let shadow_bake_pipeline = RRPipeline::new_compute(
        rrdevice,
        &WIND_SHADOW_BAKE,
        &[
            &graphics_resources.frame_set.layout,
            &shadow_bake_descriptor.layout,
        ],
    )?;

    let resolve_pipeline = fullscreen_overlay_pipeline(&WIND_RESOLVE, targets)
        .push_constants(PushConstantConfig {
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<WindPushConstants>() as u32,
        })
        .descriptor_layouts(&[
            &graphics_resources.frame_set.layout,
            &resolve_descriptor.layout,
        ])
        .build(rrdevice, rrrender, Some(targets.extent()))?;

    let upsample_descriptor = WindUpsampleDescriptorSet::new(rrdevice)?;
    upsample_descriptor.update_image_views(
        rrdevice,
        targets.half_color_image_view,
        scene_depth_view,
    )?;
    let upsample_pipeline = fullscreen_overlay_pipeline(&WIND_UPSAMPLE, targets)
        .descriptor_layouts(&[&upsample_descriptor.layout])
        .build(rrdevice, rrrender, Some(targets.extent()))?;

    log!("Created wind pipeline");
    Ok(WindGpuState {
        ubo: Some(ubo),
        resolve_pipeline: Some(resolve_pipeline),
        resolve_descriptor: Some(resolve_descriptor),
        shadow_bake_pipeline: Some(shadow_bake_pipeline),
        shadow_bake_descriptor: Some(shadow_bake_descriptor),
        upsample_pipeline: Some(upsample_pipeline),
        upsample_descriptor: Some(upsample_descriptor),
    })
}
