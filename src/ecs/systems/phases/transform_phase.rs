use anyhow::Result;
use cgmath::{Deg, InnerSpace, Vector2};

use crate::math::calculate_billboard_click_rect;
use crate::ecs::context::FrameContext;
use crate::ecs::{
    calculate_projection, gizmo_sync_position, gizmo_update_selection_color,
    gizmo_update_vertex_buffer, update_billboard_transform, update_grid_gizmo_rotation_from_view,
    ProjectionData,
};

pub unsafe fn run_transform_phase(ctx: &mut FrameContext) -> Result<()> {
    update_camera_planes(ctx);

    let proj_data = calculate_projection(&*ctx.camera(), ctx.swapchain_extent);

    let light_position = ctx.rt_debug().light_position;
    gizmo_sync_position(&mut ctx.light_gizmo_mut().position, light_position);

    {
        let mut light_gizmo = ctx.light_gizmo_mut();
        let selectable = light_gizmo.selectable.clone();
        gizmo_update_selection_color(&mut light_gizmo.mesh, &selectable);
    }
    gizmo_update_vertex_buffer(&ctx.light_gizmo().mesh, ctx.buffer_registry, ctx.device)
        .expect("Failed to update light gizmo vertex buffer");

    let camera_pos = ctx.camera().position;
    let camera_up = ctx.camera().up;
    update_billboard_transform(
        &mut ctx.billboard_mut(),
        light_position,
        camera_pos,
        camera_up,
    );

    update_grid_gizmo_rotation_from_view(&mut ctx.gizmo_mut(), proj_data.view);

    let screen_size = Vector2::new(
        ctx.swapchain_extent.0 as f32,
        ctx.swapchain_extent.1 as f32,
    );
    ctx.gui_data.billboard_click_rect = calculate_billboard_click_rect(
        light_position,
        screen_size,
        proj_data.view,
        proj_data.proj,
        0.5,
        0.1,
    );

    ctx.world.insert_resource(proj_data);

    Ok(())
}

fn update_camera_planes(ctx: &mut FrameContext) {
    let camera_distance = ctx.camera().position.magnitude();
    ctx.grid_mut().scale = 1.0;

    let grid_scale = ctx.grid().scale;
    let mut camera = ctx.camera_mut();
    camera.near_plane = (camera_distance * 0.001).max(0.1).min(10.0);
    camera.far_plane = (grid_scale * 1000.0).max(1000.0).min(100000.0);
}
