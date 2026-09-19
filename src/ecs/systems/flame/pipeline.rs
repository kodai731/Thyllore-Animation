use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::FlameGpuState;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{FlameImageBindings, RRFlameDescriptorSet, FLAME_RESOLVE};
use crate::vulkanr::pipeline::{
    BlendConfig, PipelineBuilder, PushConstantConfig, VertexInputConfig,
};
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{FlameBuffer, GraphicsResources, Placement, UniformBuffer};
use thyllore_effect_core::{FlameUBO, FLAME_MAX_INSTANCES};
use thyllore_vulkan_core::renderer::FlamePushConstants;

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

pub unsafe fn create_flame_pipeline(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &RRRender,
    graphics_resources: &GraphicsResources,
    flame_buffer: &FlameBuffer,
    position_image_view: vk::ImageView,
    position_sampler: vk::Sampler,
    scene_depth_view: vk::ImageView,
) -> Result<FlameGpuState> {
    let flame_ubo = UniformBuffer::new(
        instance,
        rrdevice,
        FLAME_MAX_INSTANCES,
        Placement::DeviceUpdated,
    )?;
    flame_ubo.write_slot(rrdevice, 0, &FlameUBO::default())?;

    let flame_descriptor = RRFlameDescriptorSet::new(rrdevice)?;
    flame_descriptor.write_all(
        rrdevice,
        &flame_ubo,
        FlameImageBindings {
            history_image_views: flame_buffer.history_image_views,
            flame_sampler: flame_buffer.sampler,
            sdf_image_view: position_image_view,
            sdf_sampler: position_sampler,
            scene_depth_view,
        },
    )?;

    let flame_shading_pipeline = PipelineBuilder::from_pass(&FLAME_RESOLVE)
        .vertex_input(VertexInputConfig::Custom {
            bindings: vec![],
            attributes: vec![],
        })
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .no_depth_test()
        .custom_render_pass(flame_buffer.shading_render_pass)
        .msaa_samples(vk::SampleCountFlags::_1)
        .mrt_attachments(2)
        .blend(premultiplied_blend())
        .attachment_blend(1, opaque_blend())
        .push_constants(PushConstantConfig {
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<FlamePushConstants>() as u32,
        })
        .dynamic_states(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
        .descriptor_layouts(&[
            &graphics_resources.frame_set.layout,
            &flame_descriptor.layout,
        ])
        .build(rrdevice, rrrender, Some(flame_buffer.extent()))?;

    log!("Created flame pipelines");
    Ok(FlameGpuState {
        shading_pipeline: Some(flame_shading_pipeline),
        descriptor: Some(flame_descriptor),
        ubo: Some(flame_ubo),
        ..Default::default()
    })
}
