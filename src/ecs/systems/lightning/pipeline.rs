use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::{LightningGpuState, LightningRenderTargets};
use crate::ecs::systems::lightning::descriptors::LightningResolveDescriptorSet;
use crate::ecs::systems::lightning::record::LightningPushConstants;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::LIGHTNING_RESOLVE;
use crate::vulkanr::pipeline::{
    BlendConfig, PipelineBuilder, PushConstantConfig, VertexInputConfig,
};
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{GraphicsResources, Placement, UniformBuffer};
use thyllore_effect_core::LIGHTNING_MAX_INSTANCES;

fn additive_blend() -> BlendConfig {
    BlendConfig {
        enable: true,
        src_color_factor: vk::BlendFactor::ONE,
        dst_color_factor: vk::BlendFactor::ONE,
        color_op: vk::BlendOp::ADD,
        src_alpha_factor: vk::BlendFactor::ZERO,
        dst_alpha_factor: vk::BlendFactor::ONE,
        alpha_op: vk::BlendOp::ADD,
    }
}

pub unsafe fn create_lightning_gpu_state(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &RRRender,
    graphics_resources: &GraphicsResources,
    targets: &LightningRenderTargets,
    scene_depth_view: vk::ImageView,
) -> Result<LightningGpuState> {
    let ubo = UniformBuffer::new(
        instance,
        rrdevice,
        LIGHTNING_MAX_INSTANCES,
        Placement::DeviceUpdated,
    )?;
    let segments_ubo = UniformBuffer::new(
        instance,
        rrdevice,
        LIGHTNING_MAX_INSTANCES,
        Placement::DeviceUpdated,
    )?;

    let resolve_descriptor = LightningResolveDescriptorSet::new(rrdevice)?;
    resolve_descriptor.write_all(rrdevice, &ubo, &segments_ubo, scene_depth_view)?;

    let resolve_pipeline = PipelineBuilder::from_pass(&LIGHTNING_RESOLVE)
        .vertex_input(VertexInputConfig::Custom {
            bindings: vec![],
            attributes: vec![],
        })
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .no_depth_test()
        .custom_render_pass(targets.render_pass)
        .msaa_samples(vk::SampleCountFlags::_1)
        .blend(additive_blend())
        .dynamic_states(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
        .push_constants(PushConstantConfig {
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<LightningPushConstants>() as u32,
        })
        .descriptor_layouts(&[
            &graphics_resources.frame_set.layout,
            &resolve_descriptor.layout,
        ])
        .build(rrdevice, rrrender, Some(targets.extent()))?;

    log!("Created lightning pipeline");
    Ok(LightningGpuState {
        ubo: Some(ubo),
        segments_ubo: Some(segments_ubo),
        resolve_pipeline: Some(resolve_pipeline),
        resolve_descriptor: Some(resolve_descriptor),
    })
}
