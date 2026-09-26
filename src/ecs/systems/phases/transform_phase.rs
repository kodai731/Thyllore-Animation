use cgmath::Vector2;

use crate::ecs::context::EcsContext;
use crate::ecs::systems::camera_systems::{compute_camera_position, compute_camera_up};
use crate::ecs::{
    calculate_projection, gizmo_sync_position, gizmo_update_selection_color,
    update_billboard_transform,
};
use crate::math::calculate_billboard_click_rect;

pub fn run_transform_phase_ecs(ctx: &mut EcsContext) {
    update_camera_near_plane(ctx);

    let proj_data = calculate_projection(&*ctx.camera(), ctx.swapchain_extent);

    let light_position = ctx.light_state().light_position;
    gizmo_sync_position(&mut ctx.light_gizmo_mut().position, light_position);

    {
        let mut light_gizmo = ctx.light_gizmo_mut();
        let selectable = light_gizmo.selectable.clone();
        if gizmo_update_selection_color(&mut light_gizmo.mesh, &selectable) {
            light_gizmo.pending_uploads = crate::ecs::MAX_FRAMES_IN_FLIGHT;
        }
    }

    let camera_pos = compute_camera_position(&ctx.camera());
    let camera_up = compute_camera_up(&ctx.camera());
    update_billboard_transform(
        &mut ctx.billboard_mut(),
        light_position,
        camera_pos,
        camera_up,
    );

    let screen_size = Vector2::new(ctx.swapchain_extent.0 as f32, ctx.swapchain_extent.1 as f32);
    let billboard_rect = calculate_billboard_click_rect(
        light_position,
        screen_size,
        proj_data.view,
        proj_data.proj,
        0.5,
        0.15,
    );
    if let Some(mut debug_view) = ctx
        .world
        .get_resource_mut::<crate::ecs::resource::DebugViewState>()
    {
        debug_view.billboard_click_rect = billboard_rect;
    }

    ctx.world.insert_resource(proj_data);
}

fn update_camera_near_plane(ctx: &mut EcsContext) {
    let camera_distance = ctx.camera().distance;
    let mut camera = ctx.camera_mut();
    camera.near_plane = (camera_distance * 0.001).max(0.1).min(10.0);
}
