use cgmath::Vector2;

use crate::ecs::context::EcsContext;
use crate::ecs::systems::camera_systems::{
    advance_camera_transition, compute_camera_position, compute_camera_up,
    tick_fly_speed_indicator, update_camera_near_plane,
};
use crate::ecs::{
    calculate_projection, gizmo_sync_position, gizmo_update_selection_color,
    update_billboard_transform,
};
use crate::math::calculate_billboard_click_rect;

pub fn run_view_phase_ecs(ctx: &mut EcsContext) {
    {
        let delta_time = ctx.delta_time;
        let mut camera = ctx.camera_mut();
        advance_camera_transition(&mut camera, delta_time);
        tick_fly_speed_indicator(&mut camera, delta_time);
        update_camera_near_plane(&mut camera);
    }

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
