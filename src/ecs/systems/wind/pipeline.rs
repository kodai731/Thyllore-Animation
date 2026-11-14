use anyhow::Result;

use crate::ecs::resource::WindGpuState;
use crate::ecs::systems::wind::descriptors::{
    create_wind_shadow_bake_sets, write_wind_shadow_bake_set, WindResolveDescriptorSet,
};
use crate::ecs::systems::wind::gpu_targets::WindRenderTargets;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{OverlayUpsampleDescriptorSet, WIND_RESOLVE, WIND_SHADOW_BAKE};
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{build_gpu_resource, GraphicsResources, Placement, UniformBuffer};
use thyllore_effect_core::{WindUBO, WIND_MAX_INSTANCES};
use thyllore_vulkan_core::renderer::{
    OverlayBlend, OverlayNodeSpec, OverlayPushConstants, ShadingPushConstants, UPSAMPLE_OVERLAY,
};
use thyllore_vulkan_core::types::{ImageView, Instance};

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
    scene_depth_view: ImageView,
    frames_in_flight: usize,
) -> Result<WindGpuState> {
    let gpu_state = build_gpu_resource(rrdevice, |gpu_state: &mut WindGpuState| {
        let ubo = gpu_state.ubo.insert(UniformBuffer::new(
            instance,
            rrdevice,
            WIND_MAX_INSTANCES,
            Placement::DeviceUpdated,
        )?);
        ubo.write_slot(rrdevice, 0, &WindUBO::default())?;

        let resolve_descriptor = gpu_state
            .resolve_descriptor
            .insert(WindResolveDescriptorSet::new(rrdevice)?);
        resolve_descriptor.write_all(rrdevice, ubo, &targets.shadow_volume, scene_depth_view)?;
        gpu_state.resolve_pipeline = Some(WIND_RESOLVE_OVERLAY.build_pipeline(
            rrdevice,
            rrrender,
            targets.render_pass(),
            &[
                &graphics_resources.frame_set.layout,
                resolve_descriptor.sets.layout(),
            ],
            targets.extent(),
        )?);

        let shadow_bake_descriptor = gpu_state
            .shadow_bake_descriptor
            .insert(create_wind_shadow_bake_sets(rrdevice)?);
        write_wind_shadow_bake_set(
            shadow_bake_descriptor,
            rrdevice,
            ubo,
            &targets.shadow_volume,
        )?;
        gpu_state.shadow_bake_pipeline = Some(RRPipeline::new_compute(
            rrdevice,
            &WIND_SHADOW_BAKE,
            &[
                &graphics_resources.frame_set.layout,
                shadow_bake_descriptor.layout(),
            ],
        )?);

        let upsample_descriptor =
            gpu_state
                .upsample_descriptor
                .insert(OverlayUpsampleDescriptorSet::new(
                    rrdevice,
                    frames_in_flight,
                )?);
        gpu_state.upsample_pipeline = Some(UPSAMPLE_OVERLAY.build_pipeline(
            rrdevice,
            rrrender,
            targets.render_pass(),
            &[upsample_descriptor.sets.layout()],
            targets.extent(),
        )?);
        Ok(())
    })?;

    log!("Created wind pipeline");
    Ok(gpu_state)
}
