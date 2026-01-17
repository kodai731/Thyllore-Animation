mod gbuffer;
mod rayquery;
mod composite;

use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
pub use gbuffer::{GBufferPass, create_gbuffer_framebuffer};
pub use rayquery::RayQueryPass;
pub use composite::CompositePass;

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

pub unsafe fn record_ray_query_pass(
    app: &App,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    let pass = RayQueryPass::new(app)?;
    let normal_offset = app.data.rt_debug_state.shadow_normal_offset;
    pass.record(command_buffer, normal_offset)
}

pub unsafe fn record_composite_pass(
    app: &mut App,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
    draw_data: &imgui::DrawData,
) -> Result<()> {
    let render_targets = app.render_targets();
    let render_pass = render_targets.render.render_pass;
    let framebuffer = render_targets.render.framebuffers[image_index];

    {
        let pass = CompositePass::new(app)?;
        pass.record(
            command_buffer,
            render_pass,
            framebuffer,
            image_index,
        )?;
    }

    app.record_imgui_rendering(command_buffer, draw_data)?;
    app.rrdevice.device.cmd_end_render_pass(command_buffer);

    Ok(())
}
