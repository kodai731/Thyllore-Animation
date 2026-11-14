use anyhow::Result;

use super::{
    create_water_caustic_apply_sets, create_water_caustic_splat_sets, create_water_resolve_sets,
    write_water_caustic_apply_set, write_water_caustic_splat_set, WaterCausticBindings,
    WaterPushConstants,
};
use crate::ecs::resource::WaterGpuState;
use crate::ecs::systems::water::gpu_targets::WaterRenderTargets;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{WATER_CAUSTIC_APPLY, WATER_CAUSTIC_SPLAT, WATER_RESOLVE};
use crate::vulkanr::pipeline::{DepthCompare, DepthTestConfig, RRPipeline};
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{
    build_gpu_resource, GraphicsResources, Placement, RayTracingData, UniformBuffer,
};
use thyllore_effect_core::{WaterUBO, WATER_MAX_INSTANCES};
use thyllore_vulkan_core::renderer::{OverlayBlend, OverlayNodeSpec, OverlayPushConstants};
use thyllore_vulkan_core::types::{ImageView, Instance};

/// Writes the shading into the HDR color and the history image where the torus is nearer than
/// the scene depth.
pub const WATER_OVERLAY: OverlayNodeSpec = OverlayNodeSpec {
    shaders: &WATER_RESOLVE,
    blends: &[OverlayBlend::Opaque, OverlayBlend::Opaque],
    depth_test: Some(DepthTestConfig::read_write(DepthCompare::GreaterOrEqual)),
    push_constants: OverlayPushConstants::of::<WaterPushConstants>(),
};

pub unsafe fn create_water_pipeline(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &RRRender,
    graphics_resources: &GraphicsResources,
    raytracing: &RayTracingData,
    water_targets: &WaterRenderTargets,
    hdr_color_view: ImageView,
    frames_in_flight: usize,
) -> Result<WaterGpuState> {
    let gpu_state = build_gpu_resource(rrdevice, |gpu_state: &mut WaterGpuState| {
        let water_ubo = gpu_state.ubo.insert(UniformBuffer::new(
            instance,
            rrdevice,
            WATER_MAX_INSTANCES,
            Placement::DeviceUpdated,
        )?);
        water_ubo.write_slot(rrdevice, 0, &WaterUBO::default())?;

        let water_descriptor = gpu_state
            .descriptor
            .insert(create_water_resolve_sets(rrdevice, frames_in_flight)?);
        gpu_state.shading_pipeline = Some(WATER_OVERLAY.build_pipeline(
            rrdevice,
            rrrender,
            water_targets.history.render_pass(),
            &[
                &graphics_resources.frame_set.layout,
                water_descriptor.layout(),
            ],
            water_targets.extent(),
        )?);

        create_water_caustic_pipelines(
            rrdevice,
            raytracing,
            gpu_state,
            water_targets,
            hdr_color_view,
        )
    })?;

    log!("Created water pipelines");
    Ok(gpu_state)
}

pub unsafe fn water_trace_blocks(
    rrdevice: &RRDevice,
    gpu_state: &WaterGpuState,
) -> Result<crate::ecs::resource::WaterTraceBlocks> {
    let Some(water_ubo) = gpu_state.ubo.as_ref() else {
        return Ok(crate::ecs::resource::WaterTraceBlocks::default());
    };
    let slot_addresses = (0..WATER_MAX_INSTANCES)
        .map(|slot| water_ubo.slot_address(rrdevice, slot))
        .collect::<Result<Vec<_>>>()?;
    Ok(crate::ecs::resource::WaterTraceBlocks::new(slot_addresses))
}

/// Caustic splat/apply need the water UBO and the water buffer, so they are built
/// once the water pipeline has produced them; the TLAS is bound later if missing.
unsafe fn create_water_caustic_pipelines(
    rrdevice: &RRDevice,
    raytracing: &RayTracingData,
    gpu_state: &mut WaterGpuState,
    water_targets: &WaterRenderTargets,
    hdr_color_view: ImageView,
) -> Result<()> {
    let (Some(gbuffer), Some(scene_buffer), Some(water_ubo)) = (
        raytracing.gbuffer.as_ref(),
        raytracing.scene_uniform_buffer_handle(),
        gpu_state.ubo.as_ref(),
    ) else {
        log!("Water caustic inputs are not ready, skipping caustic pipelines");
        return Ok(());
    };
    let tlas = raytracing
        .acceleration_structure
        .as_ref()
        .and_then(|accel| accel.tlas.acceleration_structure());

    let bindings = WaterCausticBindings {
        caustic_accum_view: water_targets.caustic_accum.view(),
        position_image_view: gbuffer.position_image_view(),
        tlas,
        scene_uniform_buffer: scene_buffer,
        water_ubo: water_ubo.handle(),
        hdr_color_image_view: hdr_color_view,
    };

    let splat_descriptor = gpu_state
        .caustic_splat_descriptor
        .insert(create_water_caustic_splat_sets(rrdevice)?);
    write_water_caustic_splat_set(splat_descriptor, rrdevice, bindings)?;
    gpu_state.caustic_splat_pipeline = Some(RRPipeline::new_compute(
        rrdevice,
        &WATER_CAUSTIC_SPLAT,
        &[splat_descriptor.layout()],
    )?);

    let apply_descriptor = gpu_state
        .caustic_apply_descriptor
        .insert(create_water_caustic_apply_sets(rrdevice)?);
    write_water_caustic_apply_set(apply_descriptor, rrdevice, bindings)?;
    gpu_state.caustic_apply_pipeline = Some(RRPipeline::new_compute(
        rrdevice,
        &WATER_CAUSTIC_APPLY,
        &[apply_descriptor.layout()],
    )?);

    gpu_state.caustic_bound_tlas = tlas.unwrap_or_default();

    log!("Created water caustic pipelines");
    Ok(())
}
