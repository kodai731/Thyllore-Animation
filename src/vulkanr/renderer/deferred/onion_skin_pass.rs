use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::PassContext;

pub unsafe fn record_onion_skin_pass(
    ctx: &PassContext,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
) -> Result<()> {
    let Some(resources) = ctx.raytracing.onion_skin_pass.as_ref() else {
        return Ok(());
    };
    let Some(onion_skin_gpu) = ctx.onion_skin_gpu else {
        return Ok(());
    };
    if onion_skin_gpu.source_mesh_index.is_none() {
        return Ok(());
    }
    if onion_skin_gpu.active_ghost_count() == 0 {
        return Ok(());
    }

    let render = ctx.frame_render_context(image_index);

    thyllore_vulkan_core::renderer::record_onion_skin_ghost_pass(
        &render,
        resources,
        onion_skin_gpu,
        image_index,
        command_buffer,
    )?;
    Ok(())
}

pub unsafe fn record_onion_skin_composite(
    ctx: &PassContext,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    let Some(resources) = ctx.raytracing.onion_skin_pass.as_ref() else {
        return Ok(());
    };
    let Some(onion_skin_gpu) = ctx.onion_skin_gpu else {
        return Ok(());
    };
    if onion_skin_gpu.source_mesh_index.is_none() {
        return Ok(());
    }
    if onion_skin_gpu.active_ghost_count() == 0 {
        return Ok(());
    }

    let render = ctx.frame_render_context(0);

    thyllore_vulkan_core::renderer::record_onion_skin_composite_pass(
        &render,
        resources,
        command_buffer,
    );
    Ok(())
}
