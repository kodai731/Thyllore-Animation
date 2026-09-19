use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::{WindGpuState, WindRenderTargets};
use crate::ecs::systems::wind::descriptors::{
    WindResolveDescriptorSet, WindShadowBakeDescriptorSet, WindUpsampleDescriptorSet,
};
use crate::ecs::systems::wind::record::WindPushConstants;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{WIND_RESOLVE, WIND_SHADOW_BAKE, WIND_UPSAMPLE};
use crate::vulkanr::pipeline::{PushConstantConfig, RRPipeline};
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{GraphicsResources, Placement, UniformBuffer};
use thyllore_effect_core::{WindUBO, WIND_MAX_INSTANCES};
use thyllore_vulkan_core::renderer::overlay_pipeline;

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

    let resolve_pipeline = overlay_pipeline(&WIND_RESOLVE, targets.render_pass)
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
    let upsample_pipeline = overlay_pipeline(&WIND_UPSAMPLE, targets.render_pass)
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
