use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::FlameGpuState;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::FLAME_RESOLVE;
use crate::vulkanr::pipeline::PushConstantConfig;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{GraphicsResources, Placement, UniformBuffer};
use thyllore_effect_core::{FlameUBO, FLAME_MAX_INSTANCES};
use thyllore_vulkan_core::renderer::{overlay_pipeline, OverlayBlend};
use thyllore_vulkan_core::resource::HistoryTargets;

use super::descriptors::{FlameImageBindings, RRFlameDescriptorSet};
use super::record::FlamePushConstants;

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

    let flame_shading_pipeline = overlay_pipeline(
        &FLAME_RESOLVE,
        flame_history.render_pass,
        &[OverlayBlend::Premultiplied, OverlayBlend::Opaque],
        None,
    )
    .push_constants(PushConstantConfig {
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        offset: 0,
        size: std::mem::size_of::<FlamePushConstants>() as u32,
    })
    .descriptor_layouts(&[
        &graphics_resources.frame_set.layout,
        &flame_descriptor.layout,
    ])
    .build(rrdevice, rrrender, Some(flame_history.extent()))?;

    log!("Created flame pipelines");
    Ok(FlameGpuState {
        shading_pipeline: Some(flame_shading_pipeline),
        descriptor: Some(flame_descriptor),
        ubo: Some(flame_ubo),
        ..Default::default()
    })
}
