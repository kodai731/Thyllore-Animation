use anyhow::Result;

use crate::ecs::resource::LightningGpuState;
use crate::ecs::systems::lightning::descriptors::LightningResolveDescriptorSet;
use crate::ecs::systems::lightning::gpu_targets::LightningRenderTargets;
use thyllore_effect_core::lightning::LIGHTNING_MAX_INSTANCES;
use thyllore_vulkan_core::core::RRDevice;
use thyllore_vulkan_core::descriptor::LIGHTNING_RESOLVE;
use thyllore_vulkan_core::render::RRRender;
use thyllore_vulkan_core::renderer::{
    OverlayBlend, OverlayNodeSpec, OverlayPushConstants, ShadingPushConstants,
};
use thyllore_vulkan_core::resource::{
    build_gpu_resource, GraphicsResources, Placement, UniformBuffer,
};
use thyllore_vulkan_core::types::{ImageView, Instance};

pub const LIGHTNING_OVERLAY: OverlayNodeSpec = OverlayNodeSpec {
    shaders: &LIGHTNING_RESOLVE,
    blends: &[OverlayBlend::Additive],
    depth_test: None,
    push_constants: OverlayPushConstants::of::<ShadingPushConstants>(),
};

pub unsafe fn create_lightning_gpu_state(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &RRRender,
    graphics_resources: &GraphicsResources,
    targets: &LightningRenderTargets,
    scene_depth_view: ImageView,
) -> Result<LightningGpuState> {
    let gpu_state = build_gpu_resource(rrdevice, |gpu_state: &mut LightningGpuState| {
        let ubo = gpu_state.ubo.insert(UniformBuffer::new(
            instance,
            rrdevice,
            LIGHTNING_MAX_INSTANCES,
            Placement::DeviceUpdated,
        )?);
        let segments_ubo = gpu_state.segments_ubo.insert(UniformBuffer::new(
            instance,
            rrdevice,
            LIGHTNING_MAX_INSTANCES,
            Placement::DeviceUpdated,
        )?);

        let resolve_descriptor = gpu_state
            .resolve_descriptor
            .insert(LightningResolveDescriptorSet::new(rrdevice)?);
        resolve_descriptor.write_all(rrdevice, ubo, segments_ubo, scene_depth_view)?;
        gpu_state.resolve_pipeline = Some(LIGHTNING_OVERLAY.build_pipeline(
            rrdevice,
            rrrender,
            targets.render_pass(),
            &[
                &graphics_resources.frame_set.layout,
                resolve_descriptor.sets.layout(),
            ],
            targets.extent(),
        )?);
        Ok(())
    })?;

    log!("Created lightning pipeline");
    Ok(gpu_state)
}
