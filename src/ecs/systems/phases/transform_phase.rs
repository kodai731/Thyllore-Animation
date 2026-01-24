use anyhow::Result;
use cgmath::{InnerSpace, Vector2};

use crate::app::FrameContext;
use crate::ecs::context::EcsContext;
use crate::ecs::{
    calculate_projection, gizmo_sync_position, gizmo_update_selection_color,
    gizmo_update_vertex_buffer, update_billboard_transform, update_grid_gizmo_rotation_from_view,
};
use crate::math::calculate_billboard_click_rect;

pub fn run_transform_phase_ecs(ctx: &mut EcsContext) {
    update_camera_planes_ecs(ctx);

    let proj_data = calculate_projection(&*ctx.camera(), ctx.swapchain_extent);

    let light_position = ctx.rt_debug().light_position;
    gizmo_sync_position(&mut ctx.light_gizmo_mut().position, light_position);

    {
        let mut light_gizmo = ctx.light_gizmo_mut();
        let selectable = light_gizmo.selectable.clone();
        gizmo_update_selection_color(&mut light_gizmo.mesh, &selectable);
    }

    let camera_pos = ctx.camera().position;
    let camera_up = ctx.camera().up;
    update_billboard_transform(
        &mut ctx.billboard_mut(),
        light_position,
        camera_pos,
        camera_up,
    );

    update_grid_gizmo_rotation_from_view(&mut ctx.gizmo_mut(), proj_data.view);

    let screen_size = Vector2::new(ctx.swapchain_extent.0 as f32, ctx.swapchain_extent.1 as f32);
    ctx.gui_data.billboard_click_rect = calculate_billboard_click_rect(
        light_position,
        screen_size,
        proj_data.view,
        proj_data.proj,
        0.5,
        0.1,
    );

    ctx.world.insert_resource(proj_data);
}

pub unsafe fn run_transform_phase_gpu(ctx: &mut FrameContext) -> Result<()> {
    let mesh = ctx.light_gizmo().mesh.clone();
    let backend = ctx.create_backend();
    gizmo_update_vertex_buffer(&mesh, &backend)
        .expect("Failed to update light gizmo vertex buffer");
    Ok(())
}

fn update_camera_planes_ecs(ctx: &mut EcsContext) {
    let camera_distance = ctx.camera().position.magnitude();
    let grid_scale = ctx.grid_scale().value();
    let mut camera = ctx.camera_mut();
    camera.near_plane = (camera_distance * 0.001).max(0.1).min(10.0);
    camera.far_plane = (grid_scale * 1000.0).max(1000.0).min(100000.0);
}
