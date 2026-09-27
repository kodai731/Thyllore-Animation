use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::{BoundGenerations, WindGpuState, WindRenderTargets};
use crate::ecs::systems::wind::descriptors::{
    WindResolveDescriptorSet, WindShadowBakeDescriptorSet,
};
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{OverlayUpsampleDescriptorSet, WIND_RESOLVE, WIND_SHADOW_BAKE};
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{GraphicsResources, Placement, UniformBuffer};
use thyllore_effect_core::{WindUBO, WIND_MAX_INSTANCES};
use thyllore_vulkan_core::renderer::{
    OverlayBlend, OverlayNodeSpec, OverlayPushConstants, ShadingPushConstants, UPSAMPLE_OVERLAY,
};

pub const WIND_RESOLVE_OVERLAY: OverlayNodeSpec = OverlayNodeSpec {
    shaders: &WIND_RESOLVE,
    blends: &[OverlayBlend::Premultiplied],
    depth_test: None,
    push_constants: OverlayPushConstants::of::<ShadingPushConstants>(),
};

pub unsafe fn create_wind_gpu_state(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &RRRender,
    graphics_resources: &GraphicsResources,
    targets: &WindRenderTargets,
    scene_depth_view: vk::ImageView,
    frames_in_flight: usize,
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
            &shadow_bake_descriptor.layout(),
        ],
    )?;

    let resolve_pipeline = WIND_RESOLVE_OVERLAY.build_pipeline(
        rrdevice,
        rrrender,
        targets.render_pass,
        &[
            &graphics_resources.frame_set.layout,
            &resolve_descriptor.layout(),
        ],
        targets.extent(),
    )?;
    let upsample_descriptor = OverlayUpsampleDescriptorSet::new(rrdevice, frames_in_flight)?;
    let upsample_pipeline = UPSAMPLE_OVERLAY.build_pipeline(
        rrdevice,
        rrrender,
        targets.render_pass,
        &[upsample_descriptor.layout()],
        targets.extent(),
    )?;
    log!("Created wind pipeline");

    Ok(WindGpuState {
        ubo: Some(ubo),
        resolve_pipeline: Some(resolve_pipeline),
        resolve_descriptor: Some(resolve_descriptor),
        shadow_bake_pipeline: Some(shadow_bake_pipeline),
        shadow_bake_descriptor: Some(shadow_bake_descriptor),
        upsample_pipeline: Some(upsample_pipeline),
        upsample_descriptor: Some(upsample_descriptor),
        upsample_bound: BoundGenerations::default(),
    })
}
