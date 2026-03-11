use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
use crate::ecs::resource::HierarchyState;
use crate::ecs::world::MeshRef;

use super::{
    AutoExposurePass, BloomPass, CompositePass, DofPass, GBufferPass, OnionSkinRenderPass,
    RayQueryPass, ToneMapPass,
};

pub unsafe fn record_gbuffer_pass(
    app: &App,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
) -> Result<()> {
    let pass = GBufferPass::new(app)?;
    let render_targets = app.render_targets();
    pass.record(
        command_buffer,
        render_targets.render.gbuffer_render_pass,
        render_targets.render.gbuffer_framebuffer,
        image_index,
    )
}

pub unsafe fn record_ray_query_pass(app: &App, command_buffer: vk::CommandBuffer) -> Result<()> {
    let pass = RayQueryPass::new(app)?;
    let normal_offset = app.light_state().shadow_normal_offset;
    pass.record(command_buffer, normal_offset)
}

fn collect_selected_mesh_ids(app: &App) -> Vec<u32> {
    let hierarchy_state = app.data.ecs_world.resource::<HierarchyState>();
    let mut selected_ids = Vec::new();

    for &entity in hierarchy_state.multi_selection.iter() {
        if let Some(mesh_ref) = app.data.ecs_world.get_component::<MeshRef>(entity) {
            if let Some(mesh_asset) = app.data.ecs_assets.get_mesh(mesh_ref.mesh_asset_id) {
                let mesh_id = (mesh_asset.graphics_mesh_index + 1) as u32;
                if !selected_ids.contains(&mesh_id) {
                    selected_ids.push(mesh_id);
                }
            }
        }
    }

    selected_ids
}

pub unsafe fn record_composite_pass(
    app: &mut App,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
    draw_data: &imgui::DrawData,
) -> Result<()> {
    let selected_mesh_ids = collect_selected_mesh_ids(app);

    if let Some(ref composite_descriptor) = app.data.raytracing.composite_descriptor {
        composite_descriptor.update_selection(&app.rrdevice, &selected_mesh_ids)?;
    }

    let render_targets = app.render_targets();
    let render_pass = render_targets.render.render_pass;
    let framebuffer = render_targets.render.framebuffers[image_index];

    {
        let pass = CompositePass::new(app)?;
        pass.record(command_buffer, render_pass, framebuffer, image_index)?;
    }

    app.record_imgui_rendering(command_buffer, draw_data)?;
    app.rrdevice.device.cmd_end_render_pass(command_buffer);

    Ok(())
}

pub unsafe fn record_composite_to_offscreen(
    app: &mut App,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
) -> Result<()> {
    let selected_mesh_ids = collect_selected_mesh_ids(app);

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

    let pass = CompositePass::new_for_offscreen(app, extent)?;
    pass.record_to_offscreen(command_buffer, render_pass, framebuffer, image_index)?;

    Ok(())
}

pub unsafe fn record_composite_to_hdr(
    app: &mut App,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    let selected_mesh_ids = collect_selected_mesh_ids(app);

    if let Some(ref composite_descriptor) = app.data.raytracing.composite_descriptor {
        composite_descriptor.update_selection(&app.rrdevice, &selected_mesh_ids)?;
    }

    let hdr_buffer = app
        .data
        .viewport
        .hdr_buffer
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("HDR buffer not initialized"))?;

    let render_pass = hdr_buffer.render_pass;
    let framebuffer = hdr_buffer.framebuffer;
    let extent = hdr_buffer.extent();

    let pass = CompositePass::new_for_offscreen(app, extent)?;
    pass.record_to_hdr(command_buffer, render_pass, framebuffer)?;

    Ok(())
}

pub unsafe fn record_bloom(app: &App, command_buffer: vk::CommandBuffer) -> Result<()> {
    let bloom_settings = app
        .data
        .ecs_world
        .get_resource::<crate::ecs::resource::BloomSettings>();
    let bloom_enabled = bloom_settings.map(|bs| bs.enabled).unwrap_or(false);

    if !bloom_enabled {
        return Ok(());
    }

    if app.data.viewport.bloom_chain.is_none()
        || app.data.raytracing.bloom_downsample_pipeline.is_none()
        || app.data.raytracing.bloom_upsample_pipeline.is_none()
    {
        return Ok(());
    }

    let pass = BloomPass::new(app)?;
    pass.record(command_buffer)?;

    Ok(())
}

pub unsafe fn record_dof(app: &App, command_buffer: vk::CommandBuffer) -> Result<()> {
    if app.data.raytracing.dof_pipeline.is_none()
        || app.data.raytracing.dof_descriptor.is_none()
        || app.data.viewport.dof_buffer.is_none()
    {
        return Ok(());
    }

    let pass = DofPass::new(app)?;
    pass.record(command_buffer)?;

    Ok(())
}

pub unsafe fn record_auto_exposure(app: &App, command_buffer: vk::CommandBuffer) -> Result<()> {
    let ae_settings = app
        .data
        .ecs_world
        .get_resource::<crate::ecs::resource::AutoExposure>();
    let ae_enabled = ae_settings.map(|ae| ae.enabled).unwrap_or(false);

    if !ae_enabled {
        return Ok(());
    }

    if app
        .data
        .raytracing
        .auto_exposure_histogram_pipeline
        .is_none()
        || app.data.raytracing.auto_exposure_average_pipeline.is_none()
        || app
            .data
            .raytracing
            .auto_exposure_histogram_descriptor
            .is_none()
        || app
            .data
            .raytracing
            .auto_exposure_average_descriptor
            .is_none()
        || app.data.viewport.auto_exposure_buffers.is_none()
    {
        return Ok(());
    }

    let pass = AutoExposurePass::new(app)?;
    pass.record(command_buffer)?;

    Ok(())
}

pub unsafe fn record_onion_skin_pass(
    app: &App,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
) -> Result<()> {
    if let Some(pass) = OnionSkinRenderPass::new(app)? {
        pass.record_ghost_pass(command_buffer, image_index)?;
    }
    Ok(())
}

pub unsafe fn record_onion_skin_composite(
    app: &App,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    if let Some(pass) = OnionSkinRenderPass::new(app)? {
        pass.record_composite_pass(command_buffer);
    }
    Ok(())
}

pub unsafe fn record_tonemap_to_offscreen(
    app: &App,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
) -> Result<()> {
    let offscreen = app
        .data
        .viewport
        .offscreen
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Offscreen framebuffer not initialized"))?;

    let render_pass = offscreen.render_pass;
    let framebuffer = offscreen.framebuffer;
    let extent = offscreen.extent();

    let pass = ToneMapPass::new(app, extent)?;
    pass.record_to_offscreen(command_buffer, render_pass, framebuffer, image_index)?;

    Ok(())
}
