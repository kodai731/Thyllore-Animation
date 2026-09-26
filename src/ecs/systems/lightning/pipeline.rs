use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::{LightningGpuState, LightningRenderTargets};
use crate::ecs::systems::lightning::descriptors::LightningResolveDescriptorSet;
use crate::ecs::systems::lightning::record::LightningPushConstants;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::LIGHTNING_RESOLVE;
use crate::vulkanr::pipeline::PushConstantConfig;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{GraphicsResources, Placement, UniformBuffer};
use thyllore_effect_core::LIGHTNING_MAX_INSTANCES;
use thyllore_vulkan_core::renderer::{overlay_pipeline, OverlayBlend};

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

    let resolve_pipeline = overlay_pipeline(
        &LIGHTNING_RESOLVE,
        targets.render_pass,
        &[OverlayBlend::Additive],
        None,
    )
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
