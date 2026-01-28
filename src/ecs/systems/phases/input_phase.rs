use anyhow::Result;
use cgmath::Vector3;

use crate::app::data::LightMoveTarget;
use crate::ecs::context::EcsContext;
use crate::ecs::GizmoAxis;
use crate::ecs::{gizmo_try_select, gizmo_update_position_with_constraint, update_light_auto_target};
use crate::math::screen_to_world_ray;

pub fn run_input_phase(ctx: &mut EcsContext) -> Result<()> {
    process_light_auto_target(ctx);

    ctx.gui_data.update();

    process_gizmo_interaction(ctx)?;

    let viewport_hovered = ctx.gui_data.viewport_hovered;
    if !ctx.light_gizmo().selectable.is_selected && viewport_hovered {
        let grid_scale = ctx.grid_scale().value();
        let is_left_clicked = ctx.gui_data.is_left_clicked;
        let is_wheel_clicked = ctx.gui_data.is_wheel_clicked;
        let mouse_wheel = ctx.gui_data.mouse_wheel;
        let mouse_diff = ctx.gui_data.mouse_diff;
        let mut camera = ctx.camera_mut();
        crate::ecs::camera_input_system_inner(
            &mut *camera,
            is_left_clicked,
            is_wheel_clicked,
            mouse_wheel,
            mouse_diff,
            grid_scale,
        );
    }

    Ok(())
}

fn process_light_auto_target(ctx: &mut EcsContext) {
    if ctx.gui_data.move_light_to == LightMoveTarget::None {
        return;
    }

    let camera_position = ctx.camera().position;
    let move_light_to = ctx.gui_data.move_light_to;
    let mut rt_debug = ctx.rt_debug_mut();
    update_light_auto_target(
        &mut *rt_debug,
        &ctx.mesh_positions,
        camera_position,
        move_light_to,
    );
    drop(rt_debug);
    ctx.gui_data.move_light_to = LightMoveTarget::None;
}

fn process_gizmo_interaction(ctx: &mut EcsContext) -> Result<()> {
    let mouse_pos = cgmath::Vector2::new(ctx.gui_data.mouse_pos[0], ctx.gui_data.mouse_pos[1]);

    if !ctx.gui_data.imgui_wants_mouse && ctx.gui_data.is_left_clicked {
        ctx.light_gizmo_mut().draggable.just_selected = false;

        let is_first_click = ctx.gui_data.clicked_mouse_pos.is_none();
        if is_first_click {
            ctx.gui_data.clicked_mouse_pos = Some([mouse_pos.x, mouse_pos.y]);

            let camera_pos = ctx.camera().position;
            let camera_dir = ctx.camera().direction;
            let camera_up = ctx.camera().up;
            {
                let mut gizmo_ref = ctx.light_gizmo_mut();
                let light_gizmo = &mut *gizmo_ref;
                let position = light_gizmo.position.clone();
                gizmo_try_select(
                    &position,
                    &mut light_gizmo.selectable,
                    &mut light_gizmo.draggable,
                    mouse_pos,
                    camera_pos,
                    camera_dir,
                    camera_up,
                    ctx.swapchain_extent,
                    ctx.gui_data.billboard_click_rect,
                );
            }
        }

        let (is_selected, just_selected) = {
            let gizmo = ctx.light_gizmo();
            (gizmo.selectable.is_selected, gizmo.draggable.just_selected)
        };

        if is_selected && ctx.gui_data.is_left_clicked && !just_selected {
            update_light_gizmo_position(ctx, mouse_pos)?;
        }
    } else if !ctx.gui_data.is_wheel_clicked {
        if ctx.gui_data.clicked_mouse_pos.is_some() {
            gizmo_handle_mouse_release(ctx);
        }
    }

    Ok(())
}

fn gizmo_handle_mouse_release(ctx: &mut EcsContext) {
    crate::log!("Mouse released - resetting light gizmo state");
    let mut gizmo = ctx.light_gizmo_mut();
    gizmo.selectable.is_selected = false;
    gizmo.selectable.selected_axis = GizmoAxis::None;
    gizmo.draggable.drag_axis = GizmoAxis::None;
    gizmo.draggable.just_selected = false;
    gizmo.draggable.initial_position = Vector3::new(0.0, 0.0, 0.0);
}

fn update_light_gizmo_position(
    ctx: &mut EcsContext,
    mouse_pos: cgmath::Vector2<f32>,
) -> Result<()> {
    use crate::math::coordinate_system::perspective;
    use cgmath::{Deg, InnerSpace};

    let camera_pos = ctx.camera().position;
    let camera_dir = ctx.camera().direction;
    let camera_up = ctx.camera().up;

    let view = unsafe { crate::math::view(camera_pos, camera_dir, camera_up) };
    let aspect = ctx.swapchain_extent.0 as f32 / ctx.swapchain_extent.1 as f32;
    let proj = perspective(Deg(45.0), aspect, 0.1, 10000.0);
    let screen_size =
        cgmath::Vector2::new(ctx.swapchain_extent.0 as f32, ctx.swapchain_extent.1 as f32);

    let (ray_origin, ray_direction) = screen_to_world_ray(mouse_pos, screen_size, view, proj);

    let light_pos = ctx.rt_debug().light_position;
    let plane_point = light_pos;
    let plane_normal = -camera_dir;

    let denom = plane_normal.dot(ray_direction);

    if denom.abs() > std::f32::EPSILON {
        let t = (plane_point - ray_origin).dot(plane_normal) / denom;

        if t >= 0.0 {
            let intersection = ray_origin + ray_direction * t;

            {
                let mut gizmo = ctx.light_gizmo_mut();
                let draggable = gizmo.draggable.clone();
                gizmo_update_position_with_constraint(
                    &mut gizmo.position,
                    intersection,
                    &draggable,
                    ctx.gui_data.is_ctrl_pressed,
                );
            }

            ctx.rt_debug_mut().light_position = ctx.light_gizmo().position.position;
        }
    }

    Ok(())
}
