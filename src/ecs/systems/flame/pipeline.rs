use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::FlameGpuState;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::FLAME_RESOLVE;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{GraphicsResources, Placement, UniformBuffer};
use thyllore_effect_core::{FlameUBO, FLAME_MAX_INSTANCES};
use thyllore_vulkan_core::renderer::{
    OverlayBlend, OverlayNodeSpec, OverlayPushConstants, ShadingPushConstants,
};
use thyllore_vulkan_core::resource::HistoryTargets;

use super::descriptors::{FlameImageBindings, RRFlameDescriptorSet};

/// Blends the shading onto the HDR color and writes the unblended result into the history image.
pub const FLAME_OVERLAY: OverlayNodeSpec = OverlayNodeSpec {
    shaders: &FLAME_RESOLVE,
    blends: &[OverlayBlend::Premultiplied, OverlayBlend::Opaque],
    depth_test: None,
    push_constants: OverlayPushConstants::of::<ShadingPushConstants>(),
};

pub unsafe fn create_flame_pipeline(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &RRRender,
    graphics_resources: &GraphicsResources,
    flame_history: &HistoryTargets,
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
            history_image_views: flame_history.views,
            flame_sampler: flame_history.sampler,
            sdf_image_view: position_image_view,
            sdf_sampler: position_sampler,
            scene_depth_view,
        },
    )?;

    let flame_shading_pipeline = FLAME_OVERLAY.build_pipeline(
        rrdevice,
        rrrender,
        flame_history.render_pass,
        &[
            &graphics_resources.frame_set.layout,
            flame_descriptor.layout(),
        ],
        flame_history.extent(),
    )?;
    log!("Created flame pipelines");

    Ok(FlameGpuState {
        shading_pipeline: Some(flame_shading_pipeline),
        descriptor: Some(flame_descriptor),
        ubo: Some(flame_ubo),
        ..Default::default()
    })
}
