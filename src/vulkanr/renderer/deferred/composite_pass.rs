use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use super::ray_query_pass::collect_selected_mesh_ids;
use crate::app::App;
use crate::ecs::{PassContext, World};
use thyllore_vulkan_core::resource::raytracing_data::RayTracingData;

fn debug_view_mode_value(mode: crate::ecs::resource::DebugViewMode) -> i32 {
    use crate::ecs::resource::DebugViewMode;
    match mode {
        DebugViewMode::Final => 0,
        DebugViewMode::Position => 1,
        DebugViewMode::Normal => 2,
        DebugViewMode::ShadowMask => 3,
        DebugViewMode::NdotL => 4,
        DebugViewMode::LightDirection => 5,
        DebugViewMode::ViewDepth => 6,
        DebugViewMode::ObjectID => 7,
        DebugViewMode::SelectionView => 8,
        DebugViewMode::SelectionUBO => 9,
    }
}

unsafe fn prepare_composite_resources<'a>(
    raytracing: &'a RayTracingData,
    world: &World,
) -> Result<(
    &'a crate::vulkanr::pipeline::RRPipeline,
    &'a crate::vulkanr::descriptor::RRCompositeDescriptorSet,
    i32,
)> {
    let pipeline = raytracing
        .composite_pipeline
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Composite pipeline not initialized"))?;
    let descriptor = raytracing
        .composite_descriptor
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Composite descriptor set not initialized"))?;
    let mode = world
        .resource::<crate::ecs::resource::DebugViewState>()
        .debug_view_mode;
    Ok((pipeline, descriptor, debug_view_mode_value(mode)))
}

pub unsafe fn record_composite_pass(
    app: &mut App,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
    draw_data: &imgui::DrawData,
) -> Result<()> {
    let selected_mesh_ids = collect_selected_mesh_ids(&app.data.ecs_world, &app.data.ecs_assets);

    if let Some(ref composite_descriptor) = app.data.raytracing.composite_descriptor {
        composite_descriptor.update_selection(&app.rrdevice, &selected_mesh_ids)?;
    }

    let render_targets = app.resource::<crate::vulkanr::context::RenderTargets>();
    let render_pass = render_targets.render.render_pass;
    let framebuffer = render_targets.render.framebuffers[image_index];
    let extent = app
        .resource::<crate::vulkanr::context::SwapchainState>()
        .swapchain
        .swapchain_extent;

    let (pipeline, descriptor, view_mode_value) =
        prepare_composite_resources(&app.data.raytracing, &app.data.ecs_world)?;

    let ctx = crate::app::build_frame_render_context(app, image_index);

    thyllore_vulkan_core::renderer::begin_composite_render_pass(
        &ctx,
        render_pass,
        framebuffer,
        extent,
        2,
        command_buffer,
    );
    thyllore_vulkan_core::renderer::record_composite_draw(
        &ctx,
        pipeline,
        descriptor,
        extent,
        view_mode_value,
        command_buffer,
    )?;
    super::OverlayRenderer::new(&ctx, &app.data.ecs_world)
        .draw_all_overlays(command_buffer, true)?;

    app.record_imgui_rendering(command_buffer, draw_data)?;
    app.rrdevice.device.cmd_end_render_pass(command_buffer);

    Ok(())
}

pub unsafe fn record_composite_to_offscreen(
    app: &mut App,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
) -> Result<()> {
    let selected_mesh_ids = collect_selected_mesh_ids(&app.data.ecs_world, &app.data.ecs_assets);

    if let Some(ref composite_descriptor) = app.data.raytracing.composite_descriptor {
        composite_descriptor.update_selection(&app.rrdevice, &selected_mesh_ids)?;
    }

    let offscreen = app
        .data
        .viewport
        .offscreen
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Offscreen framebuffer not initialized"))?;

    let render_pass = offscreen.render_pass;
    let framebuffer = offscreen.framebuffer;
    let extent = offscreen.extent();

    let (pipeline, descriptor, view_mode_value) =
        prepare_composite_resources(&app.data.raytracing, &app.data.ecs_world)?;

    let ctx = crate::app::build_frame_render_context(app, image_index);

    thyllore_vulkan_core::renderer::begin_composite_render_pass(
        &ctx,
        render_pass,
        framebuffer,
        extent,
        3,
        command_buffer,
    );
    thyllore_vulkan_core::renderer::record_composite_draw(
        &ctx,
        pipeline,
        descriptor,
        extent,
        view_mode_value,
        command_buffer,
    )?;
    super::OverlayRenderer::new(&ctx, &app.data.ecs_world)
        .draw_all_overlays(command_buffer, false)?;
    thyllore_vulkan_core::renderer::end_composite_render_pass(&ctx, command_buffer);

    Ok(())
}

pub unsafe fn record_composite_to_hdr(
    ctx: &PassContext,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    let selected_mesh_ids = collect_selected_mesh_ids(ctx.world, ctx.assets);

    if let Some(ref composite_descriptor) = ctx.raytracing.composite_descriptor {
        composite_descriptor.update_selection(ctx.rrdevice, &selected_mesh_ids)?;
    }

    let hdr_buffer = ctx
        .hdr_buffer
        .ok_or_else(|| anyhow::anyhow!("HDR buffer not initialized"))?;

    let render_pass = hdr_buffer.render_pass;
    let framebuffer = hdr_buffer.framebuffer;
    let extent = hdr_buffer.extent();
    let (pipeline, descriptor, view_mode_value) =
        prepare_composite_resources(ctx.raytracing, ctx.world)?;
    let render = ctx.frame_render_context(0);
    let black_background = ctx
        .world
        .resource::<crate::ecs::resource::DebugViewState>()
        .black_background;
    let background_radiance = if black_background {
        0.0
    } else {
        thyllore_vulkan_core::renderer::BACKGROUND_RADIANCE
    };

    thyllore_vulkan_core::renderer::begin_hdr_render_pass(
        &render,
        render_pass,
        framebuffer,
        extent,
        background_radiance,
        command_buffer,
    );

    thyllore_vulkan_core::renderer::record_composite_draw(
        &render,
        pipeline,
        descriptor,
        extent,
        view_mode_value,
        command_buffer,
    )?;

    if !black_background {
        super::OverlayRenderer::new(&render, ctx.world)
            .draw_grid_overlay(command_buffer, ctx.hdr_grid_pipeline_id)?;
    }

    thyllore_vulkan_core::renderer::end_composite_render_pass(&render, command_buffer);

    Ok(())
}
