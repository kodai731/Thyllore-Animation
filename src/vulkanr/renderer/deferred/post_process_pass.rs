use anyhow::Result;

use crate::ecs::PassContext;
use thyllore_vulkan_core::types::CommandBuffer;

struct BloomFrame<'a> {
    bloom_chain: &'a thyllore_vulkan_core::resource::BloomChain,
    downsample_pipeline: &'a thyllore_vulkan_core::pipeline::RRPipeline,
    upsample_pipeline: &'a thyllore_vulkan_core::pipeline::RRPipeline,
    descriptors: &'a thyllore_vulkan_core::descriptor::RRBloomDescriptorSets,
    targets: crate::ecs::ResRef<'a, crate::ecs::resource::PostProcessFrameTargets>,
    settings: crate::ecs::resource::BloomSettings,
}

fn bloom_frame<'a, 'b>(ctx: &'a PassContext<'b>) -> Result<Option<BloomFrame<'a>>> {
    let Some(settings) = ctx
        .world
        .get_resource::<crate::ecs::resource::BloomSettings>()
        .filter(|settings| settings.enabled)
        .map(|settings| settings.clone())
    else {
        return Ok(None);
    };
    let targets = ctx
        .world
        .resource::<crate::ecs::resource::PostProcessFrameTargets>();
    if targets.bloom_mips.is_empty() {
        return Ok(None);
    }

    let (Some(bloom_chain), Some(downsample_pipeline), Some(upsample_pipeline)) = (
        ctx.bloom_chain,
        ctx.raytracing.bloom_downsample_pipeline.as_ref(),
        ctx.raytracing.bloom_upsample_pipeline.as_ref(),
    ) else {
        return Ok(None);
    };
    let descriptors = ctx
        .raytracing
        .bloom_descriptors
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Bloom descriptors not initialized"))?;

    Ok(Some(BloomFrame {
        bloom_chain,
        downsample_pipeline,
        upsample_pipeline,
        descriptors,
        targets,
        settings,
    }))
}

pub unsafe fn record_bloom_downsample(
    ctx: &PassContext,
    command_buffer: CommandBuffer,
    mip_index: usize,
    frame_slot: usize,
) -> Result<()> {
    let Some(bloom) = bloom_frame(ctx)? else {
        return Ok(());
    };
    let render = ctx.frame_render_context(0);
    thyllore_vulkan_core::renderer::record_bloom_downsample_mip(
        &render,
        bloom.downsample_pipeline,
        bloom.descriptors,
        bloom.bloom_chain,
        &bloom.targets.bloom_mips,
        mip_index,
        frame_slot,
        &bloom.settings,
        command_buffer,
    )
}

pub unsafe fn record_bloom_upsample(
    ctx: &PassContext,
    command_buffer: CommandBuffer,
    pass_index: usize,
    frame_slot: usize,
) -> Result<()> {
    let Some(bloom) = bloom_frame(ctx)? else {
        return Ok(());
    };
    let render = ctx.frame_render_context(0);
    thyllore_vulkan_core::renderer::record_bloom_upsample_pass(
        &render,
        bloom.upsample_pipeline,
        bloom.descriptors,
        bloom.bloom_chain,
        &bloom.targets.bloom_mips,
        pass_index,
        frame_slot,
        command_buffer,
    )
}

pub unsafe fn record_dof(ctx: &PassContext, command_buffer: CommandBuffer) -> Result<()> {
    let (Some(pipeline), Some(dof_descriptor), Some(dof_buffer)) = (
        ctx.raytracing.dof_pipeline.as_ref(),
        ctx.raytracing.dof_descriptor.as_ref(),
        ctx.dof_buffer,
    ) else {
        return Ok(());
    };
    let output = ctx.transient_image(crate::app::post_process::DOF_OUTPUT)?;
    let Some(dof_framebuffer) = ctx
        .transient
        .cached_framebuffer(dof_buffer.render_pass, &[output.view])
    else {
        return Ok(());
    };

    let dof_settings = ctx
        .world
        .get_resource::<crate::ecs::resource::DepthOfField>();
    let camera_params = ctx
        .world
        .get_resource::<crate::ecs::resource::PhysicalCameraParameters>();
    let camera = ctx.world.resource::<crate::ecs::resource::Camera>();

    let dof_default = crate::ecs::resource::DepthOfField::default();
    let camera_default = crate::ecs::resource::PhysicalCameraParameters::default();

    let dof_ref: &crate::ecs::resource::DepthOfField =
        dof_settings.as_deref().unwrap_or(&dof_default);
    let camera_ref: &crate::ecs::resource::PhysicalCameraParameters =
        camera_params.as_deref().unwrap_or(&camera_default);

    let render = ctx.frame_render_context(0);

    thyllore_vulkan_core::renderer::record_dof_pass(
        &render,
        pipeline,
        dof_descriptor,
        dof_buffer,
        dof_framebuffer,
        dof_ref,
        camera_ref,
        camera.near_plane,
        command_buffer,
    )?;

    Ok(())
}

pub unsafe fn record_auto_exposure(
    ctx: &PassContext,
    command_buffer: CommandBuffer,
    frame_slot: usize,
) -> Result<()> {
    let ae_settings = ctx
        .world
        .get_resource::<crate::ecs::resource::AutoExposure>();
    let Some(ae_settings) = ae_settings else {
        return Ok(());
    };
    if !ae_settings.uses_histogram() {
        return Ok(());
    }

    let (
        Some(histogram_pipeline),
        Some(average_pipeline),
        Some(histogram_descriptor),
        Some(average_descriptor),
        Some(buffers),
    ) = (
        ctx.raytracing.auto_exposure_histogram_pipeline.as_ref(),
        ctx.raytracing.auto_exposure_average_pipeline.as_ref(),
        ctx.raytracing.auto_exposure_histogram_descriptor.as_ref(),
        ctx.raytracing.auto_exposure_average_descriptor.as_ref(),
        ctx.auto_exposure_buffers,
    )
    else {
        return Ok(());
    };

    let fixed_delta = ctx
        .world
        .resource::<crate::ecs::resource::FrameClock>()
        .fixed_delta_seconds();
    let delta_time = fixed_delta.unwrap_or_else(|| {
        ctx.world
            .get_resource::<crate::ecs::resource::TimelineState>()
            .map(|t| 1.0 / 60.0 * t.speed.max(0.01))
            .unwrap_or(1.0 / 60.0)
    });
    let render = ctx.frame_render_context(0);

    thyllore_vulkan_core::renderer::record_auto_exposure_pass(
        &render,
        histogram_pipeline,
        average_pipeline,
        histogram_descriptor,
        average_descriptor,
        buffers,
        &ae_settings,
        delta_time,
        frame_slot,
        command_buffer,
    )?;

    buffers.record_luminance_readback(render.device, command_buffer, frame_slot);

    Ok(())
}
